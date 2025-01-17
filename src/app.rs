use chrono::{self, Datelike};
use chrono_tz::Tz;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use derive_setters::Setters;
use hello_user::{ENVIRONMENT_PATH_JSON, LOG_FILE_PATH};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{
        palette::tailwind::{self},
        Color, Modifier, Style, Stylize,
    },
    symbols,
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Clear, Gauge, HighlightSpacing, Paragraph, Row, Table, Widget, Wrap,
    },
    DefaultTerminal, Frame,
};
use std::io::{BufReader, Read, Write};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
};
use tui_textarea::TextArea;

const TODO_HEADER_STYLE: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const GAUGE4_COLOR: Color = tailwind::ORANGE.c800;
const DEFAULT_TEXT_COLOR: Color = Color::Yellow;

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
pub struct App<'a> {
    /// Is the application running?
    running: bool,
    application_state: ApplicationState,
    textarea_widget: TextArea<'a>,
    running_totals: [f64; 3],
    environment_dict: serde_json::Value,
}
#[derive(Debug, Default, PartialEq)]
enum ApplicationState {
    #[default]
    Main,
    InsertRunPopup,
    InsertCalendarItemPopup,
    InsertTodoItemPopup,
}

impl App<'_> {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        self.textarea_widget = TextArea::default();
        self.environment_dict = Self::get_environment_dict();
        self.get_running_totals_from_json();
        while self.running {
            terminal.draw(|frame| self.ui(frame))?;
            self.handle_crossterm_events()?;
            match self.application_state {
                ApplicationState::InsertRunPopup => {
                    loop {
                        if let Ok(Event::Key(key_inner)) = event::read() {
                            if key_inner.code == KeyCode::Esc
                                || key_inner.modifiers == KeyModifiers::CONTROL
                                    && key_inner.code == KeyCode::Char('c')
                            {
                                let _ = append_to_log(&self.textarea_widget.lines().join("\n"));
                                break;
                            }
                            // `TextArea::input` can directly handle key events from backends and update the editor state
                            self.textarea_widget.input(key_inner);
                            terminal.draw(|frame| self.ui(frame))?;
                        } else {
                            break;
                        }
                    }
                    self.application_state = ApplicationState::Main;
                }
                ApplicationState::InsertTodoItemPopup => {
                    loop {
                        if let Ok(Event::Key(key_inner)) = event::read() {
                            if key_inner.code == KeyCode::Esc
                                || key_inner.modifiers == KeyModifiers::CONTROL
                                    && key_inner.code == KeyCode::Char('c')
                            {
                                let _ = append_to_log(&self.textarea_widget.lines().join("\n"));
                                break;
                            }
                            // `TextArea::input` can directly handle key events from backends and update the editor state
                            self.textarea_widget.input(key_inner);
                            terminal.draw(|frame| self.ui(frame))?;
                        } else {
                            break;
                        }
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn get_running_totals_from_json(&mut self) {
        if let Some(running_items) = self.environment_dict["running_totals"].as_array() {
            for (index, running_item) in running_items.iter().enumerate() {
                if let Some(running_item) = running_item.as_f64() {
                    self.running_totals[index] = running_item;
                } else {
                    let _ = append_to_log("running totals f64 conversion failed");
                }
            }
        } else {
            let _ = append_to_log("messed up running totals existing");
        }
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
            (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
                self.application_state = ApplicationState::InsertRunPopup;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('t')) => {
                self.application_state = ApplicationState::InsertRunPopup;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('0')) => {
                self.running_totals = [0.0, self.running_totals[1], self.running_totals[2]];
                let _ = self.update_running_totals_in_json();
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
        if let Some(todo_items) = self.environment_dict["todo_list"].as_array() {
            for (index, todo_item) in todo_items.iter().enumerate() {
                if let Some(todo_item_inner) = todo_item.as_str() {
                    todo_list_text.push(
                        Span::styled("\t".to_string() + todo_item_inner, todo_item_style).into(),
                    );
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
        // append_to_log(&format!("{:?}", date_to_index_map)).unwrap();
        if let Some(running_items) = self.environment_dict["running_schedule"].as_array() {
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
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD);

        let weekday_strs = [
            "月曜日",
            "火曜日",
            "水曜日",
            "木曜日",
            "金曜日",
            "土曜日",
            "日曜日",
        ];

        let today = chrono::Local::now();
        let weekday_index = today.weekday().num_days_from_monday() as usize;
        let mut weekdays_array = vec![""];
        weekdays_array.extend(weekday_strs.iter().cycle().skip(weekday_index).take(7));
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
        let row_style = Style::default().fg(Color::Yellow);
        let rows = [
            Row::new(vec!["Dawn start", "7:12"]).style(row_style),
            Row::new(vec!["Dawn end", "7:42"]).style(row_style),
            Row::new(vec!["Dusk start", "20:12"]).style(row_style),
            Row::new(vec!["Dusk end", "20:50"]).style(row_style),
            Row::new(weather_items_table).style(row_style),
            Row::new(vec!["Low", "-2°C"]).style(row_style),
            Row::new(vec!["High", "7°C"]).style(row_style),
            Row::new(am_running_items_table).style(row_style),
            Row::new(pm_running_items_table).style(row_style),
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
            .highlight_symbol(Text::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
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
        let [week_current, month_current, year_current] = self.running_totals;
        let week_max = 110.0;
        let month_max = 400.0;
        let year_max = 5000.0;
        let label_style_gauge = Style::default()
            .fg(DEFAULT_TEXT_COLOR)
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
        let shortcut_key_combination_style = Style::new().fg(Color::LightBlue);
        let important_letter_combination_styled = Style::new()
            .fg(Color::LightBlue)
            .add_modifier(Modifier::BOLD);
        let shortcut_list_lines = vec![
            vec![
                Span::styled("ctrl+r", shortcut_key_combination_style),
                Span::styled(" edit ", DEFAULT_TEXT_COLOR),
                Span::styled("r", important_letter_combination_styled),
                Span::styled("unning schedule", DEFAULT_TEXT_COLOR),
            ]
            .into(),
            vec![
                Span::styled("ctrl+t", shortcut_key_combination_style),
                Span::styled(" edit ", DEFAULT_TEXT_COLOR),
                Span::styled("t", important_letter_combination_styled).add_modifier(Modifier::BOLD),
                Span::styled("odo list", DEFAULT_TEXT_COLOR),
            ]
            .into(),
            vec![
                Span::styled("ctrl+w", shortcut_key_combination_style),
                Span::styled(" add distance to ", DEFAULT_TEXT_COLOR),
                Span::styled("w", important_letter_combination_styled),
                Span::styled("eekly total", DEFAULT_TEXT_COLOR),
            ]
            .into(),
            vec![
                Span::styled("ctrl+0", shortcut_key_combination_style),
                Span::styled(" reset weekly distance to ", DEFAULT_TEXT_COLOR),
                Span::styled("0", important_letter_combination_styled),
            ]
            .into(),
        ];
        let shortcut_list_text = Paragraph::new(shortcut_list_lines);
        ////////////
        // RENDER//
        ////////////
        match self.application_state {
            ApplicationState::InsertRunPopup => {
                let centered_area = App::center(
                    f.area(),
                    Constraint::Percentage(20),
                    Constraint::Length(3), // top and bottom border + content
                );
                self.textarea_widget.set_block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::LightBlue))
                        .title("Running Input"),
                );
                self.textarea_widget
                    .set_style(Style::default().fg(Color::Yellow));
                self.textarea_widget.set_placeholder_style(Style::default());
                self.textarea_widget.set_placeholder_text("prompt message");
                f.render_widget(Clear, centered_area);
                f.render_widget(&self.textarea_widget, centered_area);
            }
            _ => (),
        }
        f.render_widget(
            shortcut_list_text.block(
                Block::new()
                    .borders(Borders::TOP | Borders::RIGHT | Borders::LEFT)
                    .border_set(top_right_border_set),
            ),
            layout_bottom_middle[0],
        );
        f.render_widget(
            Paragraph::new("Todo plan for today with spans and calendar").block(
                Block::new()
                    .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP | Borders::BOTTOM),
            ),
            layout_left_side[0],
        );
        f.render_widget(
            Paragraph::new(todo_list_text).block(Block::new().borders(Borders::ALL)),
            layout_right[0],
        );
        f.render_widget(
            table_bottom_left
                .block(Block::new().borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM)),
            layout_left_bottom[0],
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

    fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
        let [area] = Layout::horizontal([horizontal])
            .flex(Flex::Center)
            .areas(area);
        let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
        area
    }
    fn update_running_totals_in_json(&self) -> std::io::Result<()> {
        // let mut environment_dict = App::get_environment_dict();
        // let mut file = OpenOptions::new()
        //     .write(true)
        //     .append(true)
        //     .open(ENVIRONMENT_PATH_JSON)
        //     .unwrap();

        // Ok(())

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(ENVIRONMENT_PATH_JSON)?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let mut env_dict: serde_json::Value = serde_json::from_str(&contents)?;
        if let Some(running_totals) = env_dict.get_mut("running_totals") {
            *running_totals = serde_json::json!(self.running_totals);
        } else {
            env_dict["running_totals"] = serde_json::json!(self.running_totals);
        }
        let updated_json = serde_json::to_string_pretty(&env_dict)?;
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(ENVIRONMENT_PATH_JSON)?;
        file.write_all(updated_json.as_bytes())?;

        Ok(())
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
