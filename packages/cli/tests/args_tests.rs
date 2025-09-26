use progresshub_cli::args::CliArgs;

#[test]
fn test_args_validation_valid_models() {
    let args = CliArgs {
        models: vec!["microsoft/DialoGPT-medium".to_string()],
        force: false,
        no_progress: false,
        top: false,
        bottom: false,
        quant: None,
        output_dir: None,
        token: None,
        cache_dir: None,
        verbose: false,
    };

    assert!(args.validate().is_ok());
}

#[test]
fn test_args_validation_empty_models() {
    let args = CliArgs {
        models: vec![],
        force: false,
        no_progress: false,
        top: false,
        bottom: false,
        quant: None,
        output_dir: None,
        token: None,
        cache_dir: None,
        verbose: false,
    };

    assert!(args.validate().is_err());
}

#[test]
fn test_args_validation_empty_model_name() {
    let args = CliArgs {
        models: vec!["".to_string()],
        force: false,
        no_progress: false,
        top: false,
        bottom: false,
        quant: None,
        output_dir: None,
        token: None,
        cache_dir: None,
        verbose: false,
    };

    assert!(args.validate().is_err());
}

#[test]
fn test_progress_display_preference() {
    let args_default = CliArgs {
        models: vec!["test/model".to_string()],
        force: false,
        no_progress: false,
        top: false,
        bottom: false,
        quant: None,
        output_dir: None,
        token: None,
        cache_dir: None,
        verbose: false,
    };
    assert!(args_default.should_show_progress());

    let args_no_progress = CliArgs {
        no_progress: true,
        ..args_default
    };
    assert!(!args_no_progress.should_show_progress());
}
