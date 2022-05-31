#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::style)]
#![allow(clippy::cast_precision_loss)] // u64 to f64
#![allow(clippy::cast_possible_truncation)] // u64 to usize
#![allow(clippy::cast_sign_loss)]

use clap::Parser;
use concrete_optimizer::global_parameters::DEFAUT_DOMAINS;
use concrete_optimizer::optimization::atomic_pattern as optimize_atomic_pattern;
use concrete_optimizer::optimization::wop_atomic_pattern::optimize as optimize_wop_atomic_pattern;
use rayon_cond::CondIterator;

const _4_SIGMA: f64 = 1.0 - 0.999_936_657_516;
const MIN_LOG_POLY_SIZE: u64 = DEFAUT_DOMAINS
    .glwe_pbs_constrained
    .log2_polynomial_size
    .start as u64;
const MAX_LOG_POLY_SIZE: u64 =
    DEFAUT_DOMAINS.glwe_pbs_constrained.log2_polynomial_size.end as u64 - 1;
const MAX_GLWE_DIM: u64 = DEFAUT_DOMAINS.glwe_pbs_constrained.glwe_dimension.end - 1;
const MIN_LWE_DIM: u64 = DEFAUT_DOMAINS.free_glwe.glwe_dimension.start as u64;
const MAX_LWE_DIM: u64 = DEFAUT_DOMAINS.free_glwe.glwe_dimension.end as u64 - 1;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long, default_value_t = 1, help = "1..16")]
    min_precision: u64,

    #[clap(long, default_value_t = 8, help = "1..16")]
    max_precision: u64,

    #[clap(long, default_value_t = _4_SIGMA)]
    p_error: f64,

    #[clap(long, default_value_t = 128, help = "Only 128 is supported")]
    security_level: u64,

    #[clap(long, default_value_t = MIN_LOG_POLY_SIZE, help = "8..16")]
    min_log_poly_size: u64,

    #[clap(long, default_value_t = MAX_LOG_POLY_SIZE, help = "8..16")]
    max_log_poly_size: u64,

    #[clap(long, default_value_t = 1, help = "EXPERIMENTAL")]
    min_glwe_dim: u64,

    #[clap(long, default_value_t = MAX_GLWE_DIM, help = "EXPERIMENTAL")]
    max_glwe_dim: u64,

    #[clap(long, default_value_t = MIN_LWE_DIM)]
    min_intern_lwe_dim: u64,

    #[clap(long, default_value_t = MAX_LWE_DIM)]
    max_intern_lwe_dim: u64, // 16bits needs around 1300

    #[clap(long, default_value_t = 4096)]
    sum_size: u64,

    #[clap(long)]
    no_parallelize: bool,

    #[clap(long)]
    wop_pbs: bool,
}

fn main() {
    let args = Args::parse();
    let sum_size = args.sum_size;
    let p_error = args.p_error;
    let security_level = args.security_level;
    assert!(
        security_level == 128,
        "Only 128bits of security is supported"
    );

    let glwe_log_polynomial_sizes: Vec<_> =
        (args.min_log_poly_size..=args.max_log_poly_size).collect();
    let glwe_dimensions: Vec<_> = (args.min_glwe_dim..=args.max_glwe_dim).collect();
    let internal_lwe_dimensions: Vec<_> =
        (args.min_intern_lwe_dim..=args.max_intern_lwe_dim).collect();

    let precisions = args.min_precision..=args.max_precision;
    let manps: Vec<_> = (0..=31).collect();

    // let guard = pprof::ProfilerGuard::new(100).unwrap();

    let precisions_iter = CondIterator::new(precisions.clone(), !args.no_parallelize);

    #[rustfmt::skip]
    let all_results = precisions_iter.map(|precision| {
        let mut last_solution = None;
        manps.iter().map(|&manp| {
            let noise_scale = 2_f64.powi(manp);
            let result = if args.wop_pbs {
                optimize_wop_atomic_pattern::optimize_one::<u64>(
                    sum_size,
                    precision,
                    security_level,
                    noise_scale,
                    p_error,
                    &glwe_log_polynomial_sizes,
                    &glwe_dimensions,
                    &internal_lwe_dimensions,
                    
                )
            } else {
                optimize_atomic_pattern::optimize_one::<u64>(
                    sum_size,
                    precision,
                    security_level,
                    noise_scale,
                    p_error,
                    &glwe_log_polynomial_sizes,
                    &glwe_dimensions,
                    &internal_lwe_dimensions,
                    last_solution, // 33% gains
                )
            };
            last_solution = result.best_solution;
            result
        })
        .collect::<Vec<_>>()
    })
    .collect::<Vec<_>>();

    /*
    if let Ok(report) = guard.report().build() {
        let file = std::fs::File::create("flamegraph.svg").unwrap();
        let mut options = pprof::flamegraph::Options::default();
        options.image_width = Some(32000);
        report.flamegraph_with_options(file, &mut options).unwrap();
    };
    */

    println!("{{ /* {:1.1e} errors */", p_error);
    for (precision_i, precision) in precisions.enumerate() {
        println!("{{ /* precision {:2} */", precision);
        for (manp_i, manp) in manps.clone().iter().enumerate() {
            let solution = all_results[precision_i][manp_i].best_solution;
            match solution {
                Some(solution) => {
                    println!("    /* {:2} */ V0Parameter({:2}, {:2}, {:4}, {:2}, {:2}, {:2}, {:2}), \t\t // {:4} mops, {:1.1e} errors",
                        manp, solution.glwe_dimension, (solution.glwe_polynomial_size as f64).log2() as u64,
                        solution.internal_ks_output_lwe_dimension,
                        solution.br_decomposition_level_count, solution.br_decomposition_base_log,
                        solution.ks_decomposition_level_count, solution.ks_decomposition_base_log,
                        (solution.complexity / (1024.0 * 1024.0)) as u64,
                        solution.p_error
                    );
                }
                None => {
                    println!(
                        "    /* {:2} : NO SOLUTION */ V0Parameter(0,0,0,0,0,0,0),",
                        manp
                    );
                }
            }
        }
        println!("  }},");
    }
    println!("}}");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_reference_output() {
        const REF_FILE: &str = "src/v0_parameters.ref-20-06-2022";
        const V0_PARAMETERS_EXE: &str = "../target/debug/v0-parameters";
        const CMP_LINES: &str = "\n";
        const EXACT_EQUALITY: i32 = 0;
        let _ = std::process::Command::new("cargo")
            .args(["build", "-q"])
            .status()
            .expect("Can't build");
        assert!(std::path::Path::new(V0_PARAMETERS_EXE).exists());

        let output = std::process::Command::new(V0_PARAMETERS_EXE)
            .output()
            .expect("failed to execute process");

        let actual_output = std::str::from_utf8(&output.stdout).expect("Bad content");
        let error_output = std::str::from_utf8(&output.stderr).expect("Bad content");
        assert_eq!(error_output, "");

        let expected_output = std::fs::read_to_string(REF_FILE).expect("Can't read reference file");

        text_diff::assert_diff(&expected_output, actual_output, CMP_LINES, EXACT_EQUALITY);
    }

    #[test]
    fn test_reference_wop_output() {
        const REF_FILE: &str = "src/wop_parameters.ref-20-06-2022";
        const V0_PARAMETERS_EXE: &str = "../target/release/v0-parameters";
        const CMP_LINES: &str = "\n";
        const EXACT_EQUALITY: i32 = 0;
        let _ = std::process::Command::new("cargo")
            .args(["build", "--release", "-q"])
            .status()
            .expect("Can't build");
        assert!(std::path::Path::new(V0_PARAMETERS_EXE).exists());

        #[rustfmt::skip]
        let output = std::process::Command::new(V0_PARAMETERS_EXE)
            .args([
                "--wop-pbs",
                "--min-intern-lwe-dim", "450",
                "--max-intern-lwe-dim", "600",
                "--min-precision", "1",
                "--max-precision", "16",
                "--min-log-poly-size", "10",
                "--max-log-poly-size", "11",
                "--"
                ])
            .output().expect("Failed")
            ;
        let actual_output = std::str::from_utf8(&output.stdout).expect("Bad content");
        let error_output = std::str::from_utf8(&output.stderr).expect("Bad content");

        assert_eq!(error_output, "");

        let expected_output = std::fs::read_to_string(REF_FILE).expect("Can't read reference file");

        text_diff::assert_diff(&expected_output, actual_output, CMP_LINES, EXACT_EQUALITY);
    }
}
