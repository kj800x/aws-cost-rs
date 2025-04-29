use aws_sdk_costexplorer::types::{
    DateInterval, Granularity, GroupDefinition, GroupDefinitionType,
};
use chrono::{Datelike, Duration, Local, Timelike};
use metrics::{describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;
use std::thread;

// Turns out you can't really get month-to-date usage if you check more than once daily
const DAILY_CHECK_FREQUENCY: u64 = 1;

// check equally spaced based on DAILY_CHECK_FREQUENCY
const fn sleep_delay_secs() -> u64 {
    return (60 * 60 * 24) / DAILY_CHECK_FREQUENCY;
}

// 5 minutes after the sleep delay, we expire the metric value if we haven't gotten an update
const fn metric_timeout_secs() -> u64 {
    return sleep_delay_secs() + (60 * 5);
}

fn start_of_month() -> String {
    let local = Local::now();
    let start_of_month = local
        .with_day(1)
        .expect("Failed to set day to 1") // Assuming no errors setting day to 1
        .with_hour(0)
        .expect("Failed to set hour to 0") // Assuming no errors setting hour to 0
        .with_minute(0)
        .expect("Failed to set minute to 0") // Assuming no errors setting minute to 0
        .with_second(0)
        .expect("Failed to set second to 0"); // Assuming no errors setting second to 0

    start_of_month.format("%Y-%m-%d").to_string()
}

fn tomorrow() -> String {
    let local = Local::now();
    let tomorrow = local + Duration::days(1);

    tomorrow.format("%Y-%m-%d").to_string()
}

#[::tokio::main]
async fn main() {
    let builder = PrometheusBuilder::new();
    builder
        .idle_timeout(
            MetricKindMask::ALL,
            Some(
                Duration::seconds(metric_timeout_secs().try_into().unwrap())
                    .to_std()
                    .unwrap(),
            ),
        )
        .with_http_listener(([0, 0, 0, 0], 9090))
        .install()
        .expect("Failed to install Prometheus recorder");

    describe_gauge!(
        "aws_cost_explorer",
        "The cost of an AWS service month-to-date in dollars."
    );

    let config = aws_config::load_from_env().await;
    let client = aws_sdk_costexplorer::Client::new(&config);
    println!("Hi Gelilia");

    loop {
        println!("Fetching data...");

        let response = client
            .get_cost_and_usage()
            .group_by(
                GroupDefinition::builder()
                    .set_type(Some(GroupDefinitionType::Dimension))
                    .set_key(Some("SERVICE".to_string()))
                    .build(),
            )
            .time_period(
                DateInterval::builder()
                    .set_start(Some(start_of_month()))
                    .set_end(Some(tomorrow()))
                    .build()
                    .unwrap(),
            )
            .metrics("NetUnblendedCost")
            .granularity(Granularity::Monthly)
            .send()
            .await
            .unwrap();

        if let Some(results_arr) = response.results_by_time {
            results_arr.iter().for_each(|result| {
                if let Some(groups) = result.groups.as_ref() {
                    groups.iter().for_each(|group| {
                        group
                            .metrics
                            .as_ref()
                            .unwrap()
                            .iter()
                            .for_each(|(metric, value)| {
                                gauge!(
                                    "aws_cost_explorer",
                                    value.amount.as_ref().unwrap().parse::<f64>().unwrap(),
                                    "service" => group.keys.as_ref().unwrap().join("-"),
                                    "metric" => metric.clone()
                                );
                            })
                    })
                };
            });
        };

        let sleep_duration = Duration::seconds(sleep_delay_secs().try_into().unwrap());

        // Sleep
        println!("Sleeping until {:?}...", Local::now() + sleep_duration);
        thread::sleep(sleep_duration.to_std().unwrap());
    }
}
