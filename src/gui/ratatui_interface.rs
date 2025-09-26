use color_eyre::Result;
use color_eyre::eyre::Error;
use crossterm::event::{self, Event};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use sys_info;
use tokio::task;

use crate::omikron::omikron_connection::OmikronConnection;

pub fn launch(connection_status: bool) -> Result<()> {
    color_eyre::install()?;
    task::spawn(run(connection_status));
    ratatui::restore();
    Ok(())
}

async fn run(connection_status: bool) -> Result<()> {
    loop {
        ratatui::init().draw(|frame| render(frame, connection_status))?;
    }
}

fn render(frame: &mut Frame, connection_status: bool) {
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Percentage((100))].as_ref());
    let main_block = Block::bordered().title("Tensamin - Iota");
    let main_chunks = main_layout.split(frame.area());

    let [left, right] = Layout::horizontal([Constraint::Fill(1); 2]).areas(main_chunks[0]);
    let [top_right, bottom_right] = Layout::vertical([Constraint::Fill(1); 2]).areas(right);

    let block_logs = Block::bordered().title("Logs");
    let block_systeminfo = Block::bordered().title("System Info");
    let block_ping = Block::bordered().title("Ping");

    let mut operating_system: String;
    match sys_info::os_type() {
        Ok(os) => operating_system = os,
        Err(error) => operating_system = error.to_string(),
    }

    let mut text_content: String = String::from("");
    text_content.push_str("Operating System: ");
    text_content.push_str(operating_system.as_str());
    text_content.push_str("\nConnection: ");
    text_content.push_str(if connection_status {
        "Connected"
    } else {
        "Disconnected"
    });

    let paragraph_systeminfo = Paragraph::new(text_content).block(block_systeminfo);

    frame.render_widget(Block::bordered().title("Tensamin - Iota"), frame.area());
    frame.render_widget(Block::bordered().title("Logs"), left);
    frame.render_widget(paragraph_systeminfo, top_right);
    frame.render_widget(Block::bordered().title("Ping"), bottom_right);
}
