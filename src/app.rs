use chrono::{self, Datelike};
use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{self},
};
use derive_setters::Setters;
use hello_user::LOG_FILE_PATH;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{
        palette::tailwind::{self, SLATE},
        Color, Modifier, Style, Stylize,
    },
    symbols,
    text::{Line, Span, Text},
    widgets::{
        Bar, BarChart, BarGroup, Block, Borders, Cell, Clear, Gauge, HighlightSpacing, Padding,
        Paragraph, Row, Table, Widget, Wrap,
    },
    DefaultTerminal, Frame,
};
use std::{
    collections::hash_map,
    io::{BufReader, Read, Write},
};
// use serde_json::Value;
use chrono_tz::Tz;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    process::exit,
};

const TODO_HEADER_STYLE: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const GAUGE4_COLOR: Color = tailwind::ORANGE.c800;
const GAUGE4_TEXT_COLOR: Color = Color::Yellow;

#[derive(Debug, Default, Setters)]
struct Popup<'a> {
    #[setters(into)]
    title: Line<'a>,
    #[setters(into)]
    content: Text<'a>,
    border_style: Style,
    title_style: Style,
    style: Style,
}

impl Widget for Popup<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // ensure that all cells under the popup are cleared to avoid leaking content
        Clear.render(area, buf);
        let block = Block::new()
            .title(self.title)
            .title_style(self.title_style)
            .borders(Borders::ALL)
            .border_style(self.border_style);
        Paragraph::new(self.content)
            .wrap(Wrap { trim: true })
            .style(self.style)
            .block(block)
            .render(area, buf);
    }
}

#[derive(Debug, Default)]
struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,
    colors: TableColors,
    application_state: ApplicationState,
}
#[derive(Debug, Default, PartialEq)]
enum ApplicationState {
    #[default]
    Main,
    InsertRunPopup,
    InsertCalendarItemPopup,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.ui(frame))?;
            self.handle_crossterm_events()?;
            if self.application_state != ApplicationState::Main {
                match terminal.draw(|frame| self.popup(frame)) {
                    Ok(res) => {
                        self.application_state = ApplicationState::Main;
                    }
                    Err(e) => return Err(e.into()),
                };
            }
        }
        Ok(())
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            // Add other key handlers here.
            (KeyModifiers::CONTROL, KeyCode::Char('l') | KeyCode::Char('L')) => {
                self.modify_todo_list_popup()
            }
            _ => {}
        }
    }

    fn modify_todo_list_popup(&mut self) {
        self.application_state = ApplicationState::InsertCalendarItemPopup;
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }

    fn popup(&mut self, frame: &mut Frame) {
        let popup = Popup::default()
            .content("Hello world!")
            .style(Style::new().yellow())
            .title("With Clear")
            .title_style(Style::new().white().bold())
            .border_style(Style::new().red());
        frame.render_widget(
            popup,
            Rect {
                x: 10,
                y: 10,
                width: 5,
                height: 5,
            },
        );
    }

    fn ui(&mut self, f: &mut Frame) {
        ////////////
        // LAYOUT //
        ////////////
        let vertical_percentage: u16 = 78;
        let layout_main = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints(vec![
                Constraint::Percentage(vertical_percentage),
                Constraint::Fill(1),
            ])
            .split(f.area());
        let layout_left_side = Layout::default()
            .direction(Direction::Vertical)
            .vertical_margin(0)
            .spacing(0)
            .constraints(vec![
                Constraint::Percentage(vertical_percentage),
                Constraint::Fill(1),
            ])
            .split(layout_main[0]);
        let layout_left_bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(65), Constraint::Fill(1)])
            .split(layout_left_side[1]);
        let layout_bottom_middle = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(60), Constraint::Fill(1)])
            .split(layout_left_bottom[1]);
        let layout_right = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(vertical_percentage),
                Constraint::Fill(1),
            ])
            .split(layout_main[1]);
        let layout_gauges = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ])
            .split(layout_right[1]);

        //////////////
        // DATETIME //
        //////////////
        let utc_now = chrono::Utc::now();
        let ohio_time: chrono::DateTime<Tz> = utc_now.with_timezone(&chrono_tz::US::Eastern);
        let berlin_time: chrono::DateTime<Tz> = utc_now.with_timezone(&chrono_tz::Europe::Berlin);
        let tokyo_time: chrono::DateTime<Tz> = utc_now.with_timezone(&chrono_tz::Asia::Tokyo);
        let datetime_text: Vec<Line<'_>> = vec![
            vec![Span::styled(
                ohio_time.format("%Y-%m-%d %H:%M:%S").to_string()
                    + " "
                    + &ohio_time.timezone().to_string()
                    + "     7°C  rain:80%",
                Style::default().fg(Color::Yellow),
            )]
            .into(),
            vec![Span::styled(
                berlin_time.format("%Y-%m-%d %H:%M:%S").to_string()
                    + " "
                    + &berlin_time.timezone().to_string()
                    + "  8°C  rain:50%",
                Style::default().fg(Color::Yellow),
            )]
            .into(),
            vec![Span::styled(
                tokyo_time.format("%Y-%m-%d %H:%M:%S").to_string()
                    + " "
                    + &tokyo_time.timezone().to_string()
                    + "     9°C  rain:0%",
                Style::default().fg(Color::Yellow),
            )]
            .into(),
        ];

        ////////////////
        // TODO LIST  //
        ////////////////
        let todo_item_style = Style::default().fg(Color::Yellow);
        // .bg(Color::LightCyan);
        let mut todo_list_text: Vec<Line<'_>> =
            vec![Span::styled("TODO", TODO_HEADER_STYLE).into()];
        let environment_dict = App::get_environment_dict();
        if let Some(todo_items) = environment_dict["todo_list"].as_array() {
            for (index, todo_item) in todo_items.iter().enumerate() {
                if let Some(todo_item_inner) = todo_item.as_str() {
                    todo_list_text.push(
                        Span::styled("\t".to_string() + todo_item_inner, todo_item_style).into(),
                    );
                    // println!("Todo item {}: {}", index, todo_item);
                }
            }
        } else {
            append_to_log("Todo list items don't exist").unwrap();
        }

        //////////////////////
        // RUNNING SCHEDULE //
        //////////////////////
        let mut am_running_items: Vec<&str> = vec!["rest"; 7];
        let mut pm_running_items: Vec<&str> = vec!["rest"; 7];
        let mut debug_vector: Vec<&str> = vec![];
        let mut date_to_index_map: HashMap<String, u16> = HashMap::new();
        let current_date = chrono::Local::now().naive_local().date();

        // Loop through 7 days
        for day_increment in 0..7 {
            // Calculate the new date by adding `day_increment` days
            let new_date = current_date + chrono::Duration::days(day_increment as i64);

            // Format the date as "month/day/year"
            let date_string_in_loop = new_date.format("%m/%d/%Y").to_string();

            // Insert into the map
            date_to_index_map.insert(date_string_in_loop, day_increment);
        }
        append_to_log(&format!("{:?}", date_to_index_map)).unwrap();
        if let Some(running_items) = environment_dict["running_schedule"].as_array() {
            for todo_item in running_items.iter() {
                if let Some(dict_date_key_str) = todo_item["date"].as_str() {
                    if date_to_index_map.contains_key(dict_date_key_str) {
                        if let Some(insertion_index) = date_to_index_map.get(dict_date_key_str) {
                            if let Some(current_am_string) = todo_item["am"].as_str() {
                                am_running_items[*insertion_index as usize] = current_am_string;
                            }
                            if let Some(current_pm_string) = todo_item["pm"].as_str() {
                                pm_running_items[*insertion_index as usize] = current_pm_string;
                            }
                        }
                    }
                }
            }
        } else {
            append_to_log("Running schedule items don't exist").unwrap();
        }

        ////////////////
        // TABLE STUFF//
        ////////////////
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);


        let japanese_weekdays = ["月曜日", "火曜日", "水曜日", "木曜日", "金曜日", "土曜日", "日曜日"];

        // Get today's weekday index (0 = Monday, 6 = Sunday)
        let today = chrono::Local::now();
        let weekday_index = today.weekday().num_days_from_monday() as usize;

        // Build the array
        let mut weekdays_array = vec![""]; // Start with an empty string
        weekdays_array.extend(japanese_weekdays.iter().cycle().skip(weekday_index).take(7));

        // Convert to array (if needed)
        let weekdays_array: [&str; 8] = weekdays_array.try_into().expect("Incorrect array size");

        let header = weekdays_array
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(header_style)
        .height(1);
        let bar = " █ ";

        let mut am_running_items_table = vec!["Training AM"];
        am_running_items_table.append(&mut am_running_items);
        let mut pm_running_items_table = vec!["Training PM"];
        pm_running_items_table.append(&mut pm_running_items);
        let mut weather_items_table = vec!["Weather", "Sunny"];
        weather_items_table.append(&mut debug_vector);
        let rows = [
            Row::new(vec!["Dawn start", "7:12"]),
            Row::new(vec!["Dawn end", "7:42"]),
            Row::new(vec!["Dusk start", "20:12"]),
            Row::new(vec!["Dusk end", "20:50"]),
            Row::new(weather_items_table),
            Row::new(vec!["Low", "-2°C"]),
            Row::new(vec!["High", "7°C"]),
            Row::new(am_running_items_table),
            Row::new(pm_running_items_table),
        ];
        let widths = [
            Constraint::Length(14),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ];
        let table_bottom_left = Table::new(rows, widths)
            .header(header)
            .row_highlight_style(selected_row_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(Text::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
            .bg(self.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);

        ////////////
        // WIDGETS//
        ////////////

        let top_right_border_set = symbols::border::Set {
            top_left: symbols::line::NORMAL.horizontal_down,
            ..symbols::border::PLAIN
        };
        let collapsed_top_and_left_border_set = symbols::border::Set {
            top_left: symbols::line::NORMAL.vertical_right,
            top_right: symbols::line::NORMAL.vertical_left,
            bottom_left: symbols::line::NORMAL.horizontal_up,
            ..symbols::border::PLAIN
        };
        let week_current: f64 = 15.0;
        let month_current: f64 = 200.0;
        let year_current: f64 = 200.0;
        let week_max = 110.0;
        let month_max = 400.0;
        let year_max = 5000.0;
        let label_style_gauge = Style::default()
            .fg(GAUGE4_TEXT_COLOR)
            .add_modifier(Modifier::DIM);
        let gauge_week = Gauge::default()
            .block(Block::new().borders(Borders::ALL))
            .gauge_style(GAUGE4_COLOR)
            .ratio(week_current / week_max)
            .label(Span::styled(
                week_current.to_string() + "/" + &week_max.to_string(),
                label_style_gauge,
            ));
        let gauge_month = Gauge::default()
            .gauge_style(GAUGE4_COLOR)
            .block(Block::new().borders(Borders::ALL))
            .ratio(month_current / month_max)
            .label(Span::styled(
                month_current.to_string() + "/" + &month_max.to_string(),
                label_style_gauge,
            ));
        let gauge_year = Gauge::default()
            .gauge_style(GAUGE4_COLOR)
            .block(Block::new().borders(Borders::ALL))
            .ratio(year_current / year_max)
            .label(Span::styled(
                year_current.to_string() + "/" + &year_max.to_string(),
                label_style_gauge,
            ));
        ////////////
        // RENDER//
        ////////////
        f.render_widget(
            Paragraph::new(todo_list_text).block(Block::new().borders(Borders::ALL)),
            layout_right[0],
        );
        f.render_widget(
            table_bottom_left
                .block(Block::new().borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM)),
            layout_left_bottom[0],
        );
        f.render_widget(
            Paragraph::new("Top left").block(
                Block::new()
                    .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP | Borders::BOTTOM),
            ),
            layout_left_side[0],
        );

        f.render_widget(
            Paragraph::new("Bottom middle").block(
                Block::new()
                    .borders(Borders::TOP | Borders::RIGHT | Borders::LEFT)
                    .border_set(top_right_border_set),
            ),
            layout_bottom_middle[0],
        );
        f.render_widget(gauge_week, layout_gauges[0]);
        f.render_widget(gauge_month, layout_gauges[1]);
        f.render_widget(gauge_year, layout_gauges[2]);
        f.render_widget(
            Paragraph::new(datetime_text).block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_set(collapsed_top_and_left_border_set),
            ),
            layout_bottom_middle[1],
        );
    }

    fn get_environment_dict() -> serde_json::Value {
        // let mut return_value: [[String; 7]; 2] = Default::default();
        // Read the AM for the next 7 days from the file
        let file = match fs::File::open(hello_user::ENVIRONMENT_PATH_JSON) {
            Ok(res) => res,
            Err(e) => {
                println!("{}", e.to_string());
                return Default::default();
            }
        };
        let reader = BufReader::new(file);
        let trainings_dict: serde_json::Value = match serde_json::from_reader(reader) {
            Ok(res) => res,
            Err(e) => {
                println!("{}", e.to_string());
                return Default::default();
            }
        };
        return trainings_dict;
        // let poop = trainings_dict["running_schedule"]["1/12/2005"].clone();
        // return_value[0][0] = "test".to_string();

        // return return_value;
    }
}

enum TimeOfDay {
    AM,
    PM,
}

fn append_to_log(message: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(LOG_FILE_PATH)
        .unwrap();
    if let Err(e) = writeln!(file, "{}", message) {
        eprintln!("Couldn't write to file: {}", e);
    }
    Ok(())
}
