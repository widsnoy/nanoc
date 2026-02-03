use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "testcases")]
    test_dir: Vec<PathBuf>,

    #[arg(short, long, default_value = "./target/release/airyc-compiler")]
    compiler: PathBuf,

    #[arg(short, long, default_value = "./target/release/libairyc_runtime.a")]
    runtime: PathBuf,

    #[arg(long)]
    exclude: Vec<String>,

    #[arg(long, default_value = "5")]
    timeout: u64,

    #[arg(long)]
    coverage: bool,

    #[arg(short, long)]
    verbose: bool,
}

#[derive(Clone)]
struct TestCase {
    name: String,
    path: PathBuf,
    level: String,
    has_input: bool,
}

#[derive(Debug)]
enum TestStatus {
    Passed,
    Failed(String),
    Timeout,
    CompileError(String),
}

struct TestResult {
    case: TestCase,
    status: TestStatus,
    duration: Duration,
}

fn collect_test_cases(dirs: &[PathBuf], excludes: &[String]) -> Result<Vec<TestCase>> {
    let mut cases = Vec::new();
    let excludes: Vec<&str> = excludes.iter().map(|s| s.as_str()).collect();

    fn collect_from_dir(dir: &Path, cases: &mut Vec<TestCase>, excludes: &[&str]) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_from_dir(&path, cases, excludes)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("airy") {
                let level_name = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                if excludes.contains(&level_name.as_str()) {
                    continue;
                }

                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                let has_input = path.with_extension("in").exists();
                cases.push(TestCase {
                    name,
                    path,
                    level: level_name,
                    has_input,
                });
            }
        }
        Ok(())
    }

    for dir in dirs {
        collect_from_dir(dir, &mut cases, &excludes)?;
    }

    cases.sort_by(|a, b| (&a.level, &a.name).cmp(&(&b.level, &b.name)));
    Ok(cases)
}

fn run_with_timeout(
    cmd: &mut Command,
    input: &str,
    timeout: Duration,
) -> io::Result<(Vec<u8>, i32, bool)> {
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }

    let start = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let return_code = status.code().unwrap_or(-1);
                let mut std_out = child.stdout.take().unwrap();
                let mut buf = vec![];
                let _ = std::io::Read::read_to_end(&mut std_out, &mut buf);
                return Ok((buf, return_code, false));
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    child.kill()?;
                    return Ok((vec![], -1, true));
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e),
        }
    }
}

fn run_test(case: &TestCase, args: &Args, tmp_dir: &Path) -> Result<TestResult> {
    let start = Instant::now();

    let input_file = case.path.with_extension("in");
    let input = if case.has_input {
        fs::read_to_string(&input_file)?
    } else {
        String::new()
    };

    let runtime_lib = &args.runtime;
    let compiler = &args.compiler;

    let std_out = case.path.with_extension("out");

    let std_content = match fs::read_to_string(&std_out) {
        Ok(content) => content,
        Err(_) => {
            return Ok(TestResult {
                case: case.clone(),
                status: TestStatus::CompileError(format!("{} not found", std_out.display())),
                duration: start.elapsed(),
            });
        }
    };

    let my_out = tmp_dir.join("my_out.txt");

    let timeout = Duration::from_secs(args.timeout);

    let compile_cmd = Command::new(compiler)
        .arg("-i")
        .arg(&case.path)
        .arg("-o")
        .arg(tmp_dir)
        .arg("-r")
        .arg(runtime_lib)
        .arg("-O")
        .arg("o0")
        .output()
        .context("airyc-compiler compile failed")?;

    if !compile_cmd.status.success() {
        return Ok(TestResult {
            case: case.clone(),
            status: TestStatus::CompileError(
                String::from_utf8_lossy(&compile_cmd.stderr).to_string(),
            ),
            duration: start.elapsed(),
        });
    }

    let my_exec_path = tmp_dir.join(&case.name);
    if !my_exec_path.exists() {
        return Ok(TestResult {
            case: case.clone(),
            status: TestStatus::CompileError(format!("{my_exec_path:?} not found")),
            duration: start.elapsed(),
        });
    }

    let (my_output, my_return, timed_out) =
        run_with_timeout(&mut Command::new(&my_exec_path), &input, timeout)?;

    if timed_out {
        return Ok(TestResult {
            case: case.clone(),
            status: TestStatus::Timeout,
            duration: start.elapsed(),
        });
    }

    let mut my_content = String::from_utf8_lossy(&my_output).to_string();
    my_content.push_str(&format!("return: {}\n", my_return));

    fs::write(&my_out, &my_content)?;

    let status = if std_content == my_content {
        TestStatus::Passed
    } else {
        let diff_output = Command::new("diff")
            .arg(&std_out)
            .arg(&my_out)
            .output()
            .context("diff failed")?;

        let diff = String::from_utf8_lossy(&diff_output.stdout).to_string();
        TestStatus::Failed(diff)
    };

    Ok(TestResult {
        case: case.clone(),
        status,
        duration: start.elapsed(),
    })
}

fn print_result(result: &TestResult, verbose: bool) {
    let status = match &result.status {
        TestStatus::Passed => {
            format!(
                "\x1b[32m✓\x1b[0m {} ({}ms)",
                result.case.name,
                result.duration.as_millis()
            )
        }
        TestStatus::Timeout => {
            format!("\x1b[33m⏱\x1b[0m {} - Timeout", result.case.name)
        }
        TestStatus::CompileError(e) => {
            format!("\x1b[31m✗\x1b[0m {} - Compile error: {e}", result.case.name)
        }
        TestStatus::Failed(diff) => {
            if verbose {
                format!(
                    "\x1b[31m✗\x1b[0m {} - Output mismatch\n{}",
                    result.case.name, diff
                )
            } else {
                format!("\x1b[31m✗\x1b[0m {} - Output mismatch", result.case.name)
            }
        }
    };

    println!("  [{}] {}", result.case.level, status);
}

fn print_summary(results: &[TestResult], coverage: bool) {
    let mut passed = 0;
    let mut failed = 0;
    let mut total_time = Duration::new(0, 0);

    for result in results {
        if matches!(result.status, TestStatus::Passed) {
            passed += 1;
        } else {
            failed += 1;
        }
        total_time += result.duration;
    }

    println!(
        "\nResults:\n  Passed: {}/{}\n  Failed: {}/{}\n  Time: {:.1}s\n",
        passed,
        results.len(),
        failed,
        results.len(),
        total_time.as_secs_f64()
    );

    if coverage {
        println!("Generating coverage report...");
    }
}

fn run_coverage() -> Result<()> {
    let status = Command::new("cargo")
        .args(["llvm-cov", "--workspace"])
        .status()
        .context("cargo llvm-cov failed")?;

    if !status.success() {
        anyhow::bail!("cargo llvm-cov returned non-zero status");
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let cases = collect_test_cases(&args.test_dir, &args.exclude)?;

    if cases.is_empty() {
        println!("No test cases found");
        return Ok(());
    }

    println!("Running {} tests...\n", cases.len());

    let tmp_dir = std::env::temp_dir().join("airyc-test");
    fs::create_dir_all(&tmp_dir)?;

    let mut results = Vec::new();
    let mut current_level = String::new();

    for (index, case) in cases.iter().enumerate() {
        if current_level != case.level {
            current_level = case.level.clone();
            println!("\n{}:", current_level);
        }

        let case_tmp_dir = tmp_dir.join(&case.name);
        fs::create_dir_all(&case_tmp_dir)?;

        let result = match run_test(case, &args, &case_tmp_dir) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Error running test {}: {}", case.name, e);
                TestResult {
                    case: case.clone(),
                    status: TestStatus::Failed(format!("Error: {}", e)),
                    duration: Duration::from_secs(0),
                }
            }
        };

        print_result(&result, args.verbose);
        results.push(result);

        let _ = fs::remove_dir_all(&case_tmp_dir);

        let progress = index + 1;
        print!("  [{}/{}]\r", progress, cases.len());
        io::stdout().flush()?;
    }

    println!();

    print_summary(&results, args.coverage);

    if args.coverage {
        run_coverage()?;
    }

    let failed_count = results
        .iter()
        .filter(|r| !matches!(r.status, TestStatus::Passed))
        .count();

    if failed_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}
