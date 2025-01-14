use open_meteo_api::models::TimeZone;
use open_meteo_api::query::OpenMeteo;
use std::error::Error;

// how to use

async fn get_weather_arrays(api_key: &str, lat: f32, lon: f32) -> Result<(), Box<dyn Error>> {
    // parsed json with (almost) all data you may need
    // for more info see open-meteo.com/en/docs
    // sign up to get a free api key here https://geocode.maps.co/

    let data1 = OpenMeteo::new()
        .location("London", api_key)
        .await? // add location
        .forecast_days(10)? // add forecast data
        .current_weather()? // add current weather data
        .past_days(10)? // add past days data
        .time_zone(TimeZone::EuropeLondon)? // set time zone for using .daily()
        .hourly()? // add hourly weather data
        .daily()? // add daily weather data
        .query()
        .await?;

    // using start date and end date

    let data2 = OpenMeteo::new()
        .coordinates(lat, lon)? // you can also use .coordinates(lat, lon) to set location
        .start_date("2023-09-01")?
        .end_date("2023-09-10")?
        .time_zone(TimeZone::EuropeBerlin)?
        .hourly()?
        .daily()?
        .query()
        .await?;

    // accessing data fields
    // current_weather, hourly_units, hourly, daily_units, daily have Option type
    // fields of ".hourly" and ".daily" have Vec<Option<T>> type

    let temperature = data1.current_weather.unwrap().temperature;
    let temperature_2m = data2.hourly.unwrap().temperature_2m;

    println!("{}", temperature);
    println!("{:?}", temperature_2m);

    Ok(())
}
