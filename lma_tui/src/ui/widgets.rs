use ratatui::layout::Constraint;
use ratatui::prelude::{Buffer, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, HighlightSpacing, Row, StatefulWidget, Table, TableState, Widget};

pub struct ScrollableTable<'a> {
    pub table: Table<'a>,
    row_count: usize,
    scrollbar_width: u16,
    block: Option<Block<'a>>,
}

#[allow(dead_code)]
impl<'a> ScrollableTable<'a> {
    pub fn new<T>(rows: T) -> Self
    where
        T: IntoIterator<Item = Row<'a>> + Clone,
    {
        Self {
            table: Table::new(rows.clone(), [Constraint::Percentage(100)]),
            row_count: rows.into_iter().count(),
            scrollbar_width: 1,
            block: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn header(mut self, header: Row<'a>) -> Self {
        self.table = self.table.header(header);
        self
    }

    pub fn widths(mut self, widths: &'a [Constraint]) -> Self {
        self.table = self.table.widths(widths);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.table = self.table.style(style);
        self
    }

    pub fn highlight_symbol(mut self, highlight_symbol: &'a str) -> Self {
        self.table = self.table.highlight_symbol(highlight_symbol);
        self
    }

    pub fn highlight_style(mut self, highlight_style: Style) -> Self {
        self.table = self.table.highlight_style(highlight_style);
        self
    }

    pub fn highlight_spacing(mut self, value: HighlightSpacing) -> Self {
        self.table = self.table.highlight_spacing(value);
        self
    }

    pub fn column_spacing(mut self, spacing: u16) -> Self {
        self.table = self.table.column_spacing(spacing);
        self
    }
}

impl<'a> StatefulWidget for ScrollableTable<'a> {
    type State = TableState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let spacing = 0;
        let table_area = Rect::new(
            area.x,
            area.y,
            area.width - self.scrollbar_width - spacing,
            area.height,
        );
        let table_area = self.block.take().map_or(table_area, |b| {
                let inner_area = b.inner(table_area);
                b.render(area, buf);
                inner_area
            });
        let scrollbar_area = Rect::new(
            table_area.x + table_area.width + spacing,
            table_area.y,
            self.scrollbar_width,
            table_area.height,
        );
        if self.row_count >= area.height.into() {
            let scrollbar = get_scroll_bar(state.offset(), scrollbar_area, self.row_count);
            // TODO: Make the color configurable, reimplement whole table?
            Block::default()
                .style(Style::default().bg(ratatui::style::Color::White))
                .render(scrollbar, buf);
        }
        <Table as StatefulWidget>::render(self.table, table_area, buf, state);
    }
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn get_scroll_bar(offset: usize, scrollbar_area: Rect, row_count: usize) -> Rect {
    let skipped_entries = offset as f64;
    let height = f64::from(scrollbar_area.height);
    let row_count = row_count as f64;
    let float_bar_height = height * height / row_count;
    let max_y = height - float_bar_height;
    let float_y = (skipped_entries / row_count * height).clamp(0.0, max_y);
    Rect {
        x: scrollbar_area.x,
        y: float_y as u16 + scrollbar_area.y,
        width: scrollbar_area.width,
        height: float_bar_height.ceil() as u16,
    }
}
