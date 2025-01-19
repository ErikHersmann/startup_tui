use open_meteo_api::models::TimeZone;
use open_meteo_api::query::OpenMeteo;
use std::collections::HashMap;
use std::error::Error;
use std::result;

const MISSING_VALUE_REPLACEMENT: f32 = -512.0;

// how to use

pub async fn get_weather_arrays(api_key: &str) -> Result<(), Box<dyn Error>> {
    // parsed json with (almost) all data you may need
    // for more info see open-meteo.com/en/docs
    // sign up to get a free api key here https://geocode.maps.co/

    let toledo_data = OpenMeteo::new()
        .location("Toledo", api_key)
        .await? // add location
        .current_weather()? // add daily weather data
        .time_zone(TimeZone::EuropeBerlin)?
        .forecast_days(7)?
        .daily()?
        .query()
        .await?;

    let nagoya_data = OpenMeteo::new()
        .coordinates(35.183334, 136.899994)? // you can also use .coordinates(lat, lon) to set location
        .current_weather()?
        .time_zone(TimeZone::EuropeBerlin)?
        .forecast_days(7)?
        .daily()?
        .query()
        .await?;

    // using start date and end date

    let jena_data = OpenMeteo::new()
        .coordinates(50.927223, 11.586111)? // you can also use .coordinates(lat, lon) to set location
        .forecast_days(7)?
        .current_weather()?
        .time_zone(TimeZone::EuropeBerlin)?
        .daily()?
        .query()
        .await?;

    // accessing data fields
    // current_weather, hourly_units, hourly, daily_units, daily have Option type
    // fields of ".hourly" and ".daily" have Vec<Option<T>> type

    // let temperature = data1.current_weather.unwrap().temperature;
    // let temperature_2m = data2.hourly.unwrap().temperature_2m;
    // dbg!(toledo_data);
    // dbg!(nagoya_data);
    // dbg!(jena_data);
    let mut result_hashmap: HashMap<String, Vec<String>> = HashMap::new();
    let datapack_names: Vec<String> = ["nagoya", "toledo", "jena"]
        .iter()
        .map(|item| item.to_string())
        .collect();
    for (index, data_pack) in [nagoya_data, toledo_data, jena_data].iter().enumerate() {
        if let Some(current_weather_item) = &data_pack.current_weather {
            result_hashmap.insert(
                datapack_names[index].clone() + "_current_temperature",
                vec![current_weather_item.temperature.to_string()],
            );
        }
        if let Some(daily_weather_item) = &data_pack.daily {
            let precipitation: Vec<f32> = daily_weather_item
                .precipitation_sum
                .iter()
                .map(|item| item.unwrap_or(MISSING_VALUE_REPLACEMENT))
                .collect();
            let sunrise_time: Vec<String> = daily_weather_item.sunrise.clone();
            let sunset_time: Vec<String> = daily_weather_item.sunset.clone();
            let daily_minimum_temperatures: Vec<f32> = daily_weather_item
                .temperature_2m_min
                .iter()
                .map(|item| item.unwrap_or(MISSING_VALUE_REPLACEMENT))
                .collect();
            let daily_maximum_temperatures: Vec<f32> = daily_weather_item
                .temperature_2m_max
                .iter()
                .map(|item| item.unwrap_or(MISSING_VALUE_REPLACEMENT))
                .collect();
            result_hashmap.insert(
                datapack_names[index].clone() + "_precipitation_sum",
                precipitation.iter().map(|item| {
                    if *item != MISSING_VALUE_REPLACEMENT {
                        item.to_string()
                    }
                    else {
                        "-".to_string()
                    }
                }).collect(),
            );
            result_hashmap.insert(
                datapack_names[index].clone() + "sunrise_time",
                sunrise_time
            );
            result_hashmap.insert(
                datapack_names[index].clone() + "sunset_time",
                sunset_time
            );
        }
    }

    Ok(())
}
