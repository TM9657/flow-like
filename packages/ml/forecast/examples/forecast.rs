use flow_like_ml_forecast::{ForecastFamily, ForecastModel, ForecastRequest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "Usage: forecast <timestral|chronos2|toto2|timesfm2_5|tiny_time_mixer> <checkpoint-directory> <request.json>"
                .into(),
        );
    }
    let family = match args[1].as_str() {
        "timestral" => ForecastFamily::Timestral,
        "chronos2" => ForecastFamily::Chronos2,
        "toto2" => ForecastFamily::Toto2,
        "timesfm2_5" => ForecastFamily::Timesfm25,
        "tiny_time_mixer" => ForecastFamily::TinyTimeMixer,
        _ => {
            return Err(
                "Supported families: timestral, chronos2, toto2, timesfm2_5, tiny_time_mixer"
                    .into(),
            );
        }
    };
    let request: ForecastRequest = serde_json::from_slice(&std::fs::read(&args[3])?)?;
    request.validate()?;
    let model = ForecastModel::load(
        family,
        std::path::Path::new(&args[2]),
        &burn::tensor::Device::flex(),
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&model.forecast(&request)?)?
    );
    Ok(())
}
