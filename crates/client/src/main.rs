//! calendsync-client CLI entry point.

use calendsync_client::cli::{Cli, Commands, OutputFormat};
use calendsync_client::client::calendars::{CreateCalendarRequest, UpdateCalendarRequest};
use calendsync_client::client::entries::{
    CreateEntryRequest, ListEntriesQuery, UpdateEntryRequest,
};
use calendsync_client::client::CalendsyncClient;
use calendsync_client::output::{format_output, pretty};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = CalendsyncClient::new(&cli.base_url);

    match cli.command {
        Commands::Users(users_cmd) => {
            use calendsync_client::cli::users::UsersAction;
            match users_cmd.action {
                UsersAction::List => {
                    let users = client.list_users().await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&users, cli.format)),
                        OutputFormat::Pretty => println!("{}", pretty::format_users(&users)),
                    }
                }
                UsersAction::Create { name, email } => {
                    let user = client.create_user(&name, &email).await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&user, cli.format)),
                        OutputFormat::Pretty => {
                            println!("Created:\n{}", pretty::format_user(&user))
                        }
                    }
                }
                UsersAction::Get { id } => {
                    let user = client.get_user(id).await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&user, cli.format)),
                        OutputFormat::Pretty => println!("{}", pretty::format_user(&user)),
                    }
                }
                UsersAction::Delete { id } => {
                    client.delete_user(id).await?;
                    if !cli.quiet {
                        println!("Deleted user {}", id);
                    }
                }
            }
        }
        Commands::Calendars(calendars_cmd) => {
            use calendsync_client::cli::calendars::CalendarsAction;
            match calendars_cmd.action {
                CalendarsAction::List => {
                    let calendars = client.list_calendars().await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&calendars, cli.format)),
                        OutputFormat::Pretty => {
                            println!("{}", pretty::format_calendars(&calendars))
                        }
                    }
                }
                CalendarsAction::Create {
                    name,
                    color,
                    description,
                } => {
                    let calendar = client
                        .create_calendar(CreateCalendarRequest {
                            name,
                            color: Some(color),
                            description,
                        })
                        .await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&calendar, cli.format)),
                        OutputFormat::Pretty => {
                            println!("Created:\n{}", pretty::format_calendar(&calendar))
                        }
                    }
                }
                CalendarsAction::Get { id } => {
                    let calendar = client.get_calendar(id).await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&calendar, cli.format)),
                        OutputFormat::Pretty => println!("{}", pretty::format_calendar(&calendar)),
                    }
                }
                CalendarsAction::Update {
                    id,
                    name,
                    color,
                    description,
                } => {
                    let calendar = client
                        .update_calendar(
                            id,
                            UpdateCalendarRequest {
                                name,
                                color,
                                description,
                            },
                        )
                        .await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&calendar, cli.format)),
                        OutputFormat::Pretty => {
                            println!("Updated:\n{}", pretty::format_calendar(&calendar))
                        }
                    }
                }
                CalendarsAction::Delete { id } => {
                    client.delete_calendar(id).await?;
                    if !cli.quiet {
                        println!("Deleted calendar {}", id);
                    }
                }
            }
        }
        Commands::Entries(entries_cmd) => {
            use calendsync_client::cli::entries::EntriesAction;
            match entries_cmd.action {
                EntriesAction::List {
                    calendar_id,
                    start,
                    end,
                    highlighted_day,
                    before,
                    after,
                } => {
                    let entries = client
                        .list_entries(ListEntriesQuery {
                            calendar_id,
                            start,
                            end,
                            highlighted_day,
                            before: Some(before),
                            after: Some(after),
                        })
                        .await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&entries, cli.format)),
                        OutputFormat::Pretty => println!("{}", pretty::format_entries(&entries)),
                    }
                }
                EntriesAction::Create {
                    calendar_id,
                    title,
                    date,
                    entry_type,
                    description,
                    location,
                    start_time,
                    end_time,
                    end_date,
                    color,
                } => {
                    let entry = client
                        .create_entry(CreateEntryRequest {
                            calendar_id,
                            title,
                            start_date: date,
                            entry_type: entry_type.into(),
                            description,
                            location,
                            start_time,
                            end_time,
                            end_date,
                            color,
                        })
                        .await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&entry, cli.format)),
                        OutputFormat::Pretty => {
                            println!("Created:\n{}", pretty::format_entry(&entry))
                        }
                    }
                }
                EntriesAction::Get { id } => {
                    let entry = client.get_entry(id).await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&entry, cli.format)),
                        OutputFormat::Pretty => println!("{}", pretty::format_entry(&entry)),
                    }
                }
                EntriesAction::Update {
                    id,
                    title,
                    date,
                    entry_type,
                    description,
                    location,
                    start_time,
                    end_time,
                    end_date,
                    color,
                    completed,
                } => {
                    let entry = client
                        .update_entry(
                            id,
                            UpdateEntryRequest {
                                title,
                                start_date: date,
                                entry_type: entry_type.map(Into::into),
                                description,
                                location,
                                start_time,
                                end_time,
                                end_date,
                                color,
                                completed,
                                updated_at: None, // CLI doesn't use LWW merge
                            },
                        )
                        .await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&entry, cli.format)),
                        OutputFormat::Pretty => {
                            println!("Updated:\n{}", pretty::format_entry(&entry))
                        }
                    }
                }
                EntriesAction::Delete { id } => {
                    client.delete_entry(id).await?;
                    if !cli.quiet {
                        println!("Deleted entry {}", id);
                    }
                }
                EntriesAction::Toggle { id } => {
                    let entry = client.toggle_entry(id).await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&entry, cli.format)),
                        OutputFormat::Pretty => {
                            println!("Toggled:\n{}", pretty::format_entry(&entry))
                        }
                    }
                }
            }
        }
        Commands::Events(events_cmd) => {
            use calendsync_client::cli::events::EventsAction;
            use tokio_stream::StreamExt;
            match events_cmd.action {
                EventsAction::Watch {
                    calendar_id,
                    last_event_id,
                } => {
                    if !cli.quiet {
                        println!("Watching events for calendar {}...", calendar_id);
                    }
                    let stream = client.watch_events(calendar_id, last_event_id).await?;
                    tokio::pin!(stream);
                    while let Some(event_result) = stream.next().await {
                        match event_result {
                            Ok(event) => match cli.format {
                                OutputFormat::Json => {
                                    println!("{}", serde_json::to_string(&event)?)
                                }
                                OutputFormat::Pretty => println!("{:?}", event),
                            },
                            Err(e) => {
                                eprintln!("Error: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
        }
        Commands::Health(health_cmd) => {
            use calendsync_client::cli::health::HealthAction;
            match health_cmd.action {
                HealthAction::Ssr => {
                    let health = client.health_ssr().await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&health, cli.format)),
                        OutputFormat::Pretty => {
                            println!(
                                "SSR Health:\n  Status: {}\n  Latency: {}ms",
                                health.status, health.latency_ms
                            )
                        }
                    }
                }
                HealthAction::SsrStats => {
                    let stats = client.health_ssr_stats().await?;
                    match cli.format {
                        OutputFormat::Json => println!("{}", format_output(&stats, cli.format)),
                        OutputFormat::Pretty => {
                            println!(
                                "SSR Pool Stats:\n  Workers: {}\n  Workers with capacity: {}",
                                stats.worker_count, stats.workers_with_capacity
                            )
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
