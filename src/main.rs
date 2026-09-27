use std::{io, time::Duration};
use ratatui::{Terminal, crossterm::{event::{self, Event, KeyCode}, execute, terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode}}, layout::{Constraint, Layout}, prelude::CrosstermBackend, style::{ Modifier, Style}, widgets::{Block, Borders, Tabs}};
use crate::ui::{ClientWidget, ServerWidget};

pub mod ui;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut client_widget = ClientWidget::new();
    let mut server_widget = ServerWidget::new();


    let mut tab_active = 0;

    loop{

        terminal.draw(|f|{
            let area = f.area();
            let main_layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                ])
                .split(area);

            let tabs_list = vec!["Send", "Receive"];
            let tabs = Tabs::new(tabs_list)
                .block(Block::default().borders(Borders::ALL).title("Mode"))
                .select(tab_active) 
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::BOLD),
                );

            f.render_widget(tabs, main_layout[0]);

            // navigation content
            let content_layout = main_layout[1];
            match tab_active {
                0 => client_widget.render(f, content_layout),
                1 => server_widget.render(f, content_layout),
                _ => {}
            }


            })?;

        if event::poll(Duration::from_millis(20))?{
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => {break;}
                    KeyCode::Char('s') => {
                        tab_active = 0;
                    }
                    KeyCode::Char('r') => {
                        tab_active = 1;
                    }
                    _ => {
                        // specific handler 
                        match tab_active{
                            0 => client_widget.handle_event(key.code),
                            1 => server_widget.handle_event(key.code),
                            _ => {}
                        }

                    }
                }
            }
        }
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
