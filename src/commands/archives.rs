use crate::cli::ArchivesAction;
use crate::client::Client;
use crate::error::CliError;
use crate::output;
use crate::types::*;

fn display_date_prefix(timestamp: &str) -> String {
    timestamp.chars().take(10).collect()
}

pub async fn run(action: ArchivesAction, client: &Client, json: bool) -> Result<(), CliError> {
    match action {
        ArchivesAction::Search {
            aoi,
            from,
            to,
            max_cloud,
            max_nadir,
            product_types,
            providers,
            resolutions,
            open_data,
            min_overlap,
            page,
            page_size,
        } => {
            let req = GetArchivesRequest {
                aoi,
                from_date: from,
                to_date: to,
                max_cloud_coverage_percent: max_cloud,
                max_off_nadir_angle: max_nadir,
                product_types,
                providers,
                resolutions,
                open_data,
                min_overlap_ratio: min_overlap,
                page_number: page,
                page_size,
            };
            let resp = client.post("/archives", &req).await?;
            let data: GetArchivesResponse = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                if let Some(total) = data.total {
                    eprintln!("Total results: {total}");
                }
                for archive in &data.archives {
                    println!(
                        "{:<20} {:<12} {:>8.1} km²  {:>6.2} m  ${:.2}/km²  {}",
                        archive.archive.archive_id,
                        format!("{:?}", archive.archive.provider),
                        archive.archive.total_area_square_km,
                        archive.archive.gsd,
                        archive.archive.price_for_one_square_km,
                        display_date_prefix(&archive.archive.capture_timestamp),
                    );
                }
                if let Some(next) = &data.next_page {
                    eprintln!("\nMore results available (next page token: {next})");
                }
            }
        }
        ArchivesAction::Get { archive_id } => {
            let resp = client.get(&format!("/archives/{archive_id}")).await?;
            let data: Archive = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                println!("Archive:      {}", data.archive_id);
                println!("Provider:     {:?}", data.provider);
                println!("Constellation: {}", data.constellation);
                println!("Product:      {:?}", data.product_type);
                println!("Resolution:   {} (GSD: {:.2}m)", data.resolution, data.gsd);
                println!("Captured:     {}", data.capture_timestamp);
                if let Some(cc) = data.cloud_coverage_percent {
                    println!("Cloud cover:  {cc:.1}%");
                }
                if let Some(angle) = data.off_nadir_angle {
                    println!("Off-nadir:    {angle:.1}°");
                }
                println!("Area:         {:.1} km²", data.total_area_square_km);
                println!(
                    "Price:        ${:.2}/km² (full scene: ${:.2})",
                    data.price_for_one_square_km, data.price_full_scene
                );
                println!("Min order:    {:.1} km²", data.min_sq_km);
                println!("Max order:    {:.1} km²", data.max_sq_km);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::display_date_prefix;

    #[test]
    fn display_date_prefix_limits_output_to_ten_characters() {
        assert_eq!(display_date_prefix("2025-04-01T12:00:00Z"), "2025-04-01");
    }

    #[test]
    fn display_date_prefix_is_safe_for_short_or_non_ascii_strings() {
        assert_eq!(display_date_prefix("éclair"), "éclair");
    }
}
