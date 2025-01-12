use chrono;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{palette::tailwind, Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, HighlightSpacing, Paragraph, Row, Table},
    DefaultTerminal, Frame,
};
use serde_json::Value;
use chrono_tz::Tz;
use std::{fs, io::BufReader, process::exit};

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
}

enum ApplicationState {
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
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }

    fn ui(&mut self, f: &mut Frame) {
        let layout_main = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints(vec![Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(f.area());
        let layout_left_side = Layout::default()
            .direction(Direction::Vertical)
            .vertical_margin(0)
            .spacing(0)
            .constraints(vec![Constraint::Fill(1), Constraint::Percentage(15)])
            .split(layout_main[0]);
        let layout_left_bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(65), Constraint::Fill(1)])
            .split(layout_left_side[1]);
        let layout_left_right_bottom = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(layout_left_bottom[1]);
        let styled_text = Span::styled(
            "Hello, Ratatui!",
            Style::default().fg(Color::Red).bg(Color::Yellow),
        );
        let bold_text = Span::styled(
            "This is bold",
            Style::default().add_modifier(Modifier::BOLD),
        );
        let italic_text = Span::styled(
            "This is italic",
            Style::default().add_modifier(Modifier::ITALIC),
        );
        let bold_italic_text = Span::styled(
            "This is bold and italic",
            Style::default().add_modifier(Modifier::BOLD | Modifier::ITALIC),
        );
        let mixed_line = vec![
            Span::styled("This is mixed", Style::default().fg(Color::Green)),
            Span::styled(
                " styling",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::from("!"),
        ];
        let text: Vec<Line<'_>> = vec![
            styled_text.into(),
            bold_text.into(),
            italic_text.into(),
            bold_italic_text.into(),
            mixed_line.into(),
        ];
        f.render_widget(
            Paragraph::new(text).block(Block::default().borders(Borders::ALL)),
            layout_left_side[0],
        );
        let utc_now = chrono::Utc::now();
        let ohio_time: chrono::DateTime<Tz> = utc_now.with_timezone(&chrono_tz::US::Eastern);
        let berlin_time: chrono::DateTime<Tz> = utc_now.with_timezone(&chrono_tz::Europe::Berlin);
        let tokyo_time: chrono::DateTime<Tz> = utc_now.with_timezone(&chrono_tz::Asia::Tokyo);
        let datetime_text = vec![
            vec![Span::styled(ohio_time.format("%Y-%m-%d %H:%M:%S").to_string() + " " +  &ohio_time.timezone().to_string(), Style::default().fg(Color::Yellow))].into(),
            vec![Span::styled(berlin_time.format("%Y-%m-%d %H:%M:%S").to_string() + " " +  &berlin_time.timezone().to_string(), Style::default().fg(Color::Yellow))].into(),
            vec![Span::styled(tokyo_time.format("%Y-%m-%d %H:%M:%S").to_string() + " " +  &tokyo_time.timezone().to_string(), Style::default().fg(Color::Yellow))].into(),
        ];
        // f.render_widget(
        //     Paragraph::new("Twilight/Running/Weather schedule").block(Block::default().borders(Borders::ALL)),
        //     layout_left_bottom[0],
        // );
        f.render_widget(
            Paragraph::new("今週 今月 今年").block(Block::default().borders(Borders::ALL)),
            layout_left_right_bottom[0],
        );
        f.render_widget(
            Paragraph::new(datetime_text),
            layout_left_right_bottom[1],
        );
        f.render_widget(
            Paragraph::new("Todo list").block(Block::default().borders(Borders::ALL)),
            layout_main[1],
        );

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

        let header = [
            "",
            "月曜日",
            "火曜日",
            "水曜日",
            "木曜日",
            "金曜日",
            "土曜日",
            "日曜日",
        ]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(header_style)
        .height(1);
        let bar = " █ ";

        let rows = [
            Row::new(vec!["Dawn", "7:12-7:42"]),
            Row::new(vec!["Dusk", "20:12-20:50"]),
            Row::new(vec!["Precip", "20%/30%"]),
            Row::new(vec!["Humidity", "80%"]),
            Row::new(vec!["Temperature", "7°C"]),
            Row::new(vec!["Training AM", "90 Easy"]),
            Row::new(vec!["Training PM", "Rest"]),
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
        let t = Table::new(rows, widths)
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
        f.render_widget(t, layout_left_bottom[0]);
    }

    fn get_trainings_schedule() -> [[String; 7]; 2] {
        let mut return_value: [[String; 7]; 2] = Default::default();
        // Read the AM for the next 7 days from the file
        let file = match fs::File::open(hello_user::ENVIRONMENT_PATH_JSON) {
            Ok(res) => res,
            Err(_) => exit(-9),
        };
        let reader = BufReader::new(file);
        let trainings_dict: serde_json::Value = match serde_json::from_reader(reader) {
            Ok(res) => res,
            Err(e) => exit(-9),
        };

        let poop = trainings_dict["running_schedule"]["1/12/2005"].clone();
        return_value[0][0] = "test".to_string();

        return return_value;
    }
}

enum TimeOfDay {
    AM,
    PM,
}
