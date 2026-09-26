use std::{fs::File, io::{self, Read, Write}, net::{TcpListener, TcpStream}, path::{Path, PathBuf}, sync::mpsc::{self, Receiver}, thread};

use crossterm::event::{Event, KeyCode};
use local_ip_address::local_ip;
use ratatui::{Frame, layout::{Constraint, Layout, Rect}, style::{Color, Modifier, Style}, symbols::border, widgets::{Block, Borders, List, ListItem, ListState, Paragraph}};
use tui_input::{Input, backend::crossterm::EventHandler};
use walkdir::WalkDir;

pub struct ClientWidget{
    pub list_files: Vec<PathBuf>,
    pub logs: Vec<String>,
    pub file_list_state: ListState,
    pub input: Input,
}

impl ClientWidget{
    pub fn new() ->Self{
        let files: Vec<PathBuf> = WalkDir::new(".")
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|f| f.ok())
            .filter(|f| f.file_type().is_file())
            .map(|f| f.path().to_path_buf())
            .collect();

        let mut state = ListState::default();

        if !files.is_empty(){
            state.select(Some(0));
        }

        Self { 
            list_files: files,
            logs: vec![], 
            file_list_state: state,
            input: Input::default()
        }
    }

    fn send_file(server_address: String, file_path: String) -> io::Result<()> {
        let mut stream = TcpStream::connect(server_address)?;
        let file_name = Path::new(&file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?;

        let file_name_bytes = file_name.as_bytes();
        let file_name_len = file_name_bytes.len() as u32;

        // send length filename (4 bytes, Big Endian)
        stream.write_all(&file_name_len.to_be_bytes())?;

        // send string filename
        stream.write_all(file_name_bytes)?;

        let mut file = File::open(file_path)?;
        let mut buffer = [0u8; 65536];

        while let Ok(n) = file.read(&mut buffer) {
            if n == 0 { break; }
            stream.write_all(&buffer[..n])?;
        }
        stream.flush()?;
        Ok(())
    }

    pub fn render(&mut self, f: &mut Frame, layout: Rect ){
            let l = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(5),
                    Constraint::Length(3),
                ])
                .split(layout);

            let input_widget = Paragraph::new(self.input.value())
                .style(Style::default().fg(Color::Yellow))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" IP Server ")
                        .border_set(ratatui::symbols::border::ROUNDED)
                );
            f.render_widget(input_widget, l[0]);

            f.set_cursor_position((
                l[0].x + 1 + self.input.visual_cursor() as u16,
                l[0].y + 1,
            ));

            let items: Vec<ListItem> = self.list_files
                .iter()
                .map(|f|{
                    ListItem::new(f.display().to_string())
                }).collect();
            let content_widget = List::new(items)
            .block(Block::default()
                .title("Files")
                .border_set(border::ROUNDED)
                .borders(Borders::ALL)
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

            f.render_stateful_widget(content_widget, l[1], &mut self.file_list_state);

            let logs: Vec<ListItem> = self.logs
                .iter()
                .rev()
                .map(|x|{
                    ListItem::new(x.to_string())
                }).collect();
            let queue_log_widget = List::new(logs)
            .block(Block::default()
                .title("Logs")
                .border_set(border::ROUNDED)
                .borders(Borders::ALL)
            );
            f.render_widget(queue_log_widget, l[2] );
            let footer_widget = Paragraph::new(" [Esc] Exit  |  [c] Client | [s] Sender | [Enter] Send | [Ctrl+Backspace] Backspace")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer_widget, l[3]);
    }

    pub fn handle_event(&mut self, key: KeyCode){
        match key{
            KeyCode::Enter =>{
                if let Some(select_file_idx) = self.file_list_state.selected(){
                    let path_selected = &self.list_files[select_file_idx];
                    let file_path = path_selected.display().to_string();

                    let (tx, rx) = mpsc::channel();
                    let addr = format!("{}:5001", self.input.value());

                    if self.input.value().is_empty(){
                        self.logs.push("Please insert ip address server".to_string());
                    }else{
                        self.logs.push(format!("Sending file: {}", file_path));
                        thread::spawn(move||{
                            match ClientWidget::send_file(addr, file_path){
                                Ok(_) => {
                                    let _ = tx.send(String::from("Files is sended"));
                                }
                                Err(_)=>{
                                    let _ = tx.send(String::from("Failed send files"));
                                }
                            }
                        });

                        if let  Ok(msg) = rx.try_recv(){
                            self.logs.push(msg);
                        }
                    }


                };

            }
            KeyCode::Down =>{
                if self.list_files.is_empty() { return; }
                
                let i = match self.file_list_state.selected() {
                    Some(i) => {
                        if i >= self.list_files.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.file_list_state.select(Some(i));
            }
            KeyCode::Up => {
                if self.list_files.is_empty() { return; }
                
                let i = match self.file_list_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.list_files.len() - 1 
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.file_list_state.select(Some(i));
            }
            _ => {
                self.input.handle_event(&Event::Key(key.into()));
            }
        }
    }
}


pub struct ServerWidget{
    pub list_files: Vec<PathBuf>,
    pub logs: Vec<String>,
    pub file_list_state: ListState,
    pub server_rx: Receiver<String>
}

impl ServerWidget{
    pub fn new() ->Self{
        let mut logs = vec![];
        let files: Vec<PathBuf> = WalkDir::new(".")
            .max_depth(1)
            .into_iter()
            .filter_map(|f| f.ok())
            .map(|f| f.path().to_path_buf())
            .collect();

        let mut state = ListState::default();

        if !files.is_empty(){
            state.select(Some(0));
        }

        let addr = String::from("0.0.0.0:5001");
        logs.push(format!("Server running at {}", addr));
        let rx = Self::start_server(addr);

        Self { list_files: files, logs, file_list_state: state, server_rx: rx}
    }

    fn start_server(addr: String) -> Receiver<String>{
        let (tx, rx) = mpsc::channel::<String>(); 
        let tx_clone = tx.clone(); 

        // move tcp serve into thread so the server not blocking render
        let addr_c = addr.clone();
        thread::spawn(move || {
            let listener = match TcpListener::bind(addr_c) {
                Ok(l) => l,
                Err(_) => {
                    let _ = tx_clone.send(String::from("Error: Failed bind port!"));
                    return;
                }
            };

            let _ = tx_clone.send(String::from("Server Active. Waiting client..."));
            loop {
                // listening inbound connection
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut len_buffer = [0u8; 4];
                        if stream.read_exact(&mut len_buffer).is_err() {
                            let _ = tx_clone.send(String::from("Error: Failed read metadata!"));
                            continue;
                        }
                        let file_name_len = u32::from_be_bytes(len_buffer) as usize;

                         
                        let mut name_buffer = vec![0u8; file_name_len];
                        if stream.read_exact(&mut name_buffer).is_err() {
                            let _ = tx_clone.send(String::from("Failed read filename!"));
                            continue;
                        }
                        let file_name = match String::from_utf8(name_buffer) {
                            Ok(name) => name,
                            Err(_) => {
                                let _ = tx_clone.send(String::from("Filename is not valid UTF-8"));
                                continue;
                            }
                        };

                        let _ = tx_clone.send(format!("Transfering: {}", file_name));

                        let mut file = match File::create(&file_name) {
                            Ok(f) => f,
                            Err(_) => {
                                let _ = tx_clone.send(format!("Failed create {} file", file_name));
                                continue;
                            }
                        };
                        
                        let mut buffer = [0u8; 65536];
                        while let Ok(n) = stream.read(&mut buffer) {
                            if n == 0 { break; }
                            if file.write_all(&buffer[..n]).is_err() {
                                let _ = tx_clone.send(String::from("Failed create file!"));
                                break;
                            }
                        }

                        let _ = tx_clone.send(format!("File '{}' received!", file_name));
                    }
                    Err(_) => {
                        let _ = tx_clone.send(String::from("Failed receive connection!"));
                    }
                }   
            }
        });


        rx
    }

    pub fn render(&mut self, f: &mut Frame, layout: Rect ){
        while let Ok(msg) = self.server_rx.try_recv() {
            self.logs.push(msg)
        }
        let l = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(5),
                Constraint::Length(3),
            ])
            .split(layout);
        let ip_local = local_ip().unwrap();
        let header_widget = Paragraph::new(format!("IP: {}", ip_local.to_string()))
        .block(Block::default()
            .border_set(border::ROUNDED)
            .borders(Borders::ALL
        ));
        f.render_widget(header_widget, l[0]);

        let items: Vec<ListItem> = self.list_files
            .iter()
            .map(|f|{
                ListItem::new(f.display().to_string())
            }).collect();
        let content_widget = List::new(items)
        .block(Block::default()
            .title("Server Files")
            .border_set(border::ROUNDED)
            .borders(Borders::ALL)
        );

        f.render_stateful_widget(content_widget, l[1], &mut self.file_list_state);

        let logs: Vec<ListItem> = self.logs
            .iter()
            .rev()
            .map(|x|{
                ListItem::new(x.to_string())
            }).collect();
        let queue_log_widget = List::new(logs)
        .block(Block::default()
            .title("Logs")
            .border_set(border::ROUNDED)
            .borders(Borders::ALL)
        );
        f.render_widget(queue_log_widget, l[2] );

        let footer_widget = Paragraph::new(" [Esc] Exit  |  [c] Client | [s] Sender")
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer_widget, l[3]);
    }

    pub fn handle_event(&self, key: KeyCode){
        match key{
            KeyCode::Enter =>{
            }
            _ => {}
        }
    }
}
