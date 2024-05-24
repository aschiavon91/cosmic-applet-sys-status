use crate::window::Message;
use chrono::{DateTime, Utc};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::Size;
use cosmic::iced_renderer::Geometry;
use cosmic::iced_widget::canvas::Frame;
use cosmic::iced_widget::Column;
use cosmic::widget::Text;
use cosmic::Element;
use cosmic::{
    iced::{
        widget::{Row, Scrollable},
        Alignment, Length,
    },
    iced_widget::canvas::Cache,
};
use plotters::prelude::*;
use plotters_iced::{Chart, ChartBuilder, ChartWidget};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
use sysinfo::{CpuRefreshKind, RefreshKind, System};

const PLOT_SECONDS: usize = 60;
const SAMPLE_EVERY: Duration = Duration::from_millis(1000);

pub struct SystemChart {
    sys: System,
    last_sample_time: Instant,
    cpu: Option<PercentualUsageChart>,
    memory: Option<PercentualUsageChart>,
    chart_height: f32,
    color: RGBColor,
}

impl SystemChart {
    pub fn new(color: RGBColor) -> Self {
        Self {
            sys: System::new_with_specifics(
                RefreshKind::new()
                    .with_cpu(CpuRefreshKind::new().with_cpu_usage())
                    .without_processes(),
            ),
            color,
            last_sample_time: Instant::now(),
            chart_height: 180.0,
            cpu: None,
            memory: None,
        }
    }
}

impl SystemChart {
    #[inline]
    fn is_initialized(&self) -> bool {
        self.cpu.is_some()
    }

    #[inline]
    fn should_update(&self) -> bool {
        !self.is_initialized() || self.last_sample_time.elapsed() > SAMPLE_EVERY
    }

    pub fn update(&mut self) {
        if !self.should_update() {
            return;
        }

        self.sys.refresh_all();
        self.last_sample_time = Instant::now();
        let now = Utc::now();
        let cpu_data = self.sys.global_cpu_info().cpu_usage() as i32;
        let total_memory = self.sys.total_memory() as f64;
        let used_memory = self.sys.used_memory() as f64;
        let memory_data = ((used_memory / total_memory) * 100.0) as i32;

        //check if initialized
        if !self.is_initialized() {
            self.cpu = Some(PercentualUsageChart::new(
                vec![(now, cpu_data)].into_iter(),
                self.color,
            ));
            self.memory = Some(PercentualUsageChart::new(
                vec![(now, memory_data)].into_iter(),
                self.color,
            ));
        } else {
            self.cpu
                .as_mut()
                .expect("uninitialzed cpu error")
                .push_data(now, cpu_data);

            self.memory
                .as_mut()
                .expect("uninitialzed memory error")
                .push_data(now, memory_data);
        }
    }

    pub fn view(&self) -> Element<Message> {
        if !self.is_initialized() {
            Text::new("Loading...")
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center)
                .into()
        } else {
            // let chart_height = self.chart_height;
            // let mut idx = 0;
            // for chunk in self.processors.chunks(self.items_per_row) {
            //     let mut row = Row::new()
            //         .spacing(8)
            //         .padding(12)
            //         .width(Length::Fill)
            //         .height(Length::Shrink)
            //         .align_items(Alignment::Center);
            //     for item in chunk {
            //         row = row.push(item.view(idx, chart_height));
            //         idx += 1;
            //     }
            //     while idx % self.items_per_row != 0 {
            //         row = row.push(Space::new(Length::Fill, Length::Fixed(50.0)));
            //         idx += 1;
            //     }
            //     col = col.push(row);
            // }

            let cpu_chart = self.cpu.as_ref().unwrap().view("CPU", self.chart_height);
            let cpu_row = Row::with_children(vec![cpu_chart])
                .spacing(8)
                .padding(12)
                .width(Length::Fill)
                .height(Length::Shrink)
                .align_items(Alignment::Center);

            let memory_chart = self
                .memory
                .as_ref()
                .unwrap()
                .view("Memory", self.chart_height);
            let memory_row = Row::with_children(vec![memory_chart])
                .spacing(8)
                .padding(12)
                .width(Length::Fill)
                .height(Length::Shrink)
                .align_items(Alignment::Center);

            let col = Column::with_children(vec![cpu_row.into(), memory_row.into()])
                .width(Length::Fill)
                .height(Length::Shrink)
                .align_items(Alignment::Center);

            Scrollable::new(col).height(Length::Shrink).into()
        }
    }
}

struct PercentualUsageChart {
    cache: Cache,
    data_points: VecDeque<(DateTime<Utc>, i32)>,
    limit: Duration,
    color: RGBColor,
}

impl PercentualUsageChart {
    fn new(data: impl Iterator<Item = (DateTime<Utc>, i32)>, color: RGBColor) -> Self {
        let data_points: VecDeque<_> = data.collect();
        Self {
            cache: Cache::new(),
            data_points,
            limit: Duration::from_secs(PLOT_SECONDS as u64),
            color,
        }
    }

    fn push_data(&mut self, time: DateTime<Utc>, value: i32) {
        let cur_ms = time.timestamp_millis();
        self.data_points.push_front((time, value));
        loop {
            if let Some((time, _)) = self.data_points.back() {
                let diff = Duration::from_millis((cur_ms - time.timestamp_millis()) as u64);
                if diff > self.limit {
                    self.data_points.pop_back();
                    continue;
                }
            }
            break;
        }
        self.cache.clear();
    }

    fn view(&self, title: &str, chart_height: f32) -> Element<Message> {
        Column::new()
            .width(Length::Fill)
            .height(Length::Shrink)
            .spacing(5)
            .align_items(Alignment::Center)
            .push(Text::new(title.to_string()))
            .push(ChartWidget::new(self).height(Length::Fixed(chart_height)))
            .into()
    }
}

impl Chart<Message> for PercentualUsageChart {
    type State = ();

    #[inline]
    fn draw<R: plotters_iced::Renderer, F: Fn(&mut Frame)>(
        &self,
        renderer: &R,
        bounds: Size,
        draw_fn: F,
    ) -> Geometry {
        renderer.draw_cache(&self.cache, bounds, draw_fn)
    }

    fn build_chart<DB: DrawingBackend>(&self, _state: &Self::State, mut chart: ChartBuilder<DB>) {
        // Acquire time range
        let newest_time = self
            .data_points
            .front()
            .unwrap_or(&(chrono::DateTime::from_timestamp(0, 0).unwrap(), 0))
            .0;
        let oldest_time = newest_time - chrono::Duration::seconds(PLOT_SECONDS as i64);
        let mut chart = chart
            .x_label_area_size(0)
            .y_label_area_size(28)
            .margin(20)
            .build_cartesian_2d(oldest_time..newest_time, 0..100)
            .expect("failed to build chart");

        chart
            .configure_mesh()
            .bold_line_style(self.color.mix(0.1))
            .light_line_style(self.color.mix(0.05))
            .axis_style(ShapeStyle::from(self.color.mix(0.45)).stroke_width(1))
            .y_labels(10)
            .y_label_style(
                ("sans-serif", 8)
                    .into_font()
                    .color(&self.color.mix(0.65))
                    .transform(FontTransform::Rotate90),
            )
            .y_label_formatter(&y_label_formatter)
            .draw()
            .expect("failed to draw chart mesh");

        chart
            .draw_series(
                AreaSeries::new(
                    self.data_points.iter().map(|x| (x.0, x.1)),
                    0,
                    self.color.mix(0.175),
                )
                .border_style(ShapeStyle::from(self.color).stroke_width(1)),
            )
            .expect("failed to draw chart data");
    }
}

fn y_label_formatter(v: &i32) -> String {
    return format!("{}%", v);
}
