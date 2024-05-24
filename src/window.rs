use crate::chart;
use crate::chart::SystemChart;
use crate::config::{Config, CONFIG_VERSION};
use cosmic::app::Core;
use cosmic::cosmic_theme::palette::WithAlpha;
use cosmic::iced::wayland::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::{self, Command, Limits};
use cosmic::iced::{Alignment, Length};
use cosmic::iced_futures::Subscription;
use cosmic::iced_style::application;
use cosmic::Element;
use cosmic::Theme;
use cosmic::{cosmic_config, widget};
use cosmic_time::Duration;
use plotters::style::RGBColor;

pub const ID: &str = "app.arara.CosmicAppletSysStatus";

pub struct Window {
    core: Core,
    config: Config,
    #[allow(dead_code)]
    config_handler: Option<cosmic_config::Config>,
    popup: Option<Id>,
    icon_name: String,
    chart: chart::SystemChart,
}

#[derive(Clone, Debug)]
pub enum Message {
    Config(Config),
    TogglePopup,
    Tick,
}

#[derive(Clone, Debug)]
pub struct Flags {
    pub config_handler: Option<cosmic_config::Config>,
    pub config: Config,
}

impl cosmic::Application for Window {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = Flags;
    type Message = Message;
    const APP_ID: &'static str = ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(
        core: Core,
        flags: Self::Flags,
    ) -> (Self, Command<cosmic::app::Message<Self::Message>>) {
        let config = flags.config;

        let theme = core.applet.theme().expect("applet them could not be get");
        let accent_color = theme
            .cosmic()
            .accent_color()
            .into_format::<u8, u8>()
            .without_alpha();
        let chart_color = RGBColor(accent_color.red, accent_color.green, accent_color.blue);
        println!("{:?}", accent_color);

        let window = Window {
            core,
            config,
            config_handler: flags.config_handler,
            popup: None,
            icon_name: ID.to_string(),
            chart: SystemChart::new(chart_color),
        };

        (window, Command::none())
    }

    fn update(&mut self, message: Self::Message) -> Command<cosmic::app::Message<Self::Message>> {
        // Helper for updating config values efficiently
        #[allow(unused_macros)]
        macro_rules! config_set {
            ($name: ident, $value: expr) => {
                match &self.config_handler {
                    Some(config_handler) => {
                        match paste::paste! { self.config.[<set_ $name>](config_handler, $value) } {
                            Ok(_) => {}
                            Err(err) => {
                                eprintln!("failed to save config {:?}: {}", stringify!($name), err);
                            }
                        }
                    }
                    None => {
                        self.config.$name = $value;
                        eprintln!(
                            "failed to save config {:?}: no config handler",
                            stringify!($name),
                        );
                    }
                }
            };
        }

        match message {
            Message::Tick => self.chart.update(),
            Message::Config(config) => {
                if config != self.config {
                    self.config = config
                }
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings =
                        self.core
                            .applet
                            .get_popup_settings(Id::MAIN, new_id, None, None, None);
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(475.0)
                        .min_width(300.0)
                        .min_height(200.0)
                        .max_height(1080.0);
                    get_popup(popup_settings)
                }
            }
        }

        Command::none()
    }

    fn view<'a>(&'a self) -> Element<Self::Message> {
        self.core
            .applet
            .icon_button(&self.icon_name)
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        #[allow(unused_variables)]
        let cosmic::cosmic_theme::Spacing {
            space_none, // 0
            space_xxxs, // 4
            space_xxs,  // 8
            space_xs,   // 12
            space_s,    // 16
            space_m,    // 24
            space_l,    // 32
            space_xl,   // 48
            space_xxl,  // 64
            space_xxxl, // 128
        } = self.core.system_theme().cosmic().spacing;

        // let mut cols = widget::column::with_capacity(2).width(Length::Fill);

        // let cpu_info = self.system.cpus().first().unwrap().brand();
        // let cpu_usage = self.system.global_cpu_info().cpu_usage();
        // cols = cols
        //     .push(backend::cpu_widget(cpu_info, cpu_usage))
        //     .push(backend::memory_widget(
        //         self.system.used_memory(),
        //         self.system.total_memory(),
        //     ));

        // let mut labels = self
        //     .components
        //     .iter()
        //     .map(|v| (v.label().to_string(), v.temperature()))
        //     .collect::<Vec<(String, f32)>>();

        // labels.sort_by(|(a, _), (b, _)| a.cmp(b));

        // for (label, temp) in labels {
        //     cols = cols.push(
        //         widget::text(format!("{} {}°C", label, temp.trunc() as u32))
        //             .apply(widget::container)
        //             .padding(12)
        //             .apply(Element::from),
        //     );
        // }

        let content = widget::column()
            .spacing(10)
            .align_items(Alignment::Start)
            .width(Length::Shrink)
            .height(Length::Shrink)
            .push(self.chart.view());

        let chart_container = widget::container(content)
            .width(Length::Fill)
            .height(Length::Shrink)
            .padding(5)
            .center_x()
            .center_y();

        // let content = widget::column::with_children(vec![cols.into(), chart_container.into()])
        //     .padding([space_xxs, space_xxxs])
        //     .spacing(space_m);

        self.core.applet.popup_container(chart_container).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        const FPS: u64 = 60;
        let ticks = iced::time::every(Duration::from_millis(1000 / FPS))
            .map(|_| Message::Tick)
            .map(|_| Message::Tick);

        struct ConfigSubscription;
        let config = cosmic_config::config_subscription(
            std::any::TypeId::of::<ConfigSubscription>(),
            Self::APP_ID.into(),
            CONFIG_VERSION,
        )
        .map(|update| {
            if !update.errors.is_empty() {
                eprintln!(
                    "errors loading config {:?}: {:?}",
                    update.keys, update.errors
                );
            }
            Message::Config(update.config)
        });

        Subscription::batch(vec![config, ticks])
    }

    fn style(&self) -> Option<<Theme as application::StyleSheet>::Style> {
        Some(cosmic::applet::style())
    }
}
