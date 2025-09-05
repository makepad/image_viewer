use makepad_widgets::*;
use moly_kit::{
    ChatTask, ChatWidgetRefExt, OpenAIClient, protocol::*,
    utils::asynchronous::spawn,
};
use std::path::{Path, PathBuf};

use crate::slideshow_client::SlideshowClient;

live_design! {
    use link::widgets::*;
    use moly_kit::widgets::chat::Chat;

    LEFT_ARROW = dep("crate://self/resources/left_arrow.svg");
    RIGHT_ARROW = dep("crate://self/resources/right_arrow.svg");
    PLACEHOLDER = dep("crate://self/resources/placeholder.png");

    MenuBarButton = <Button> {
        text: "Slideshow",
    }

    MenuBar = <View> {
        width: Fill,
        height: Fit,
        align: {
            x: 1.0,
        },

        button = <MenuBarButton> {}
    }

    ImageGridItem = <View> {
        width: 256,
        height: 256,
        align: {
            x: 0.5,
            y: 0.5,
        },

        <View> {
            animator: {
                hover = {
                    default: off,

                    off = {
                        from: {
                            all: Forward {
                                duration: 0.1,
                            },
                        },
                        apply: {
                            width: 230,
                            height: 230,
                        },
                        redraw: true,
                    }

                    on = {
                        from: {
                            all: Forward {
                                duration: 0.1,
                            },
                        },
                        apply: {
                            width: 256,
                            height: 256,
                        },
                        redraw: true,
                    }
                }
            }

            image = <Image> {
                width: Fill,
                height: Fill,
                fit: Biggest,
                source: (PLACEHOLDER),
            }
        }
    }

    ImageGridRow = {{ImageGridRow}} {
        items = <PortalList> {
            height: 256,
            flow: Right,

            scroll_bar: {
                draw_bg: {
                    fn pixel(self) -> vec4 {
                        return vec4(0.0, 0.0, 0.0, 0.0);
                    }
                },
            },

            Item = <ImageGridItem> {}
        }
    }

    ImageGrid = {{ImageGrid}} {
        rows = <PortalList> {
            flow: Down,

            scroll_bar: {
                draw_bg: {
                    fn pixel(self) -> vec4 {
                        return vec4(0.0, 0.0, 0.0, 0.0);
                    }
                },
            },

            Row = <ImageGridRow> {}
        }
    }

    ImageBrowser = <View> {
        flow: Down,

        menu_bar = <MenuBar> {}
        image_grid = <ImageGrid> {}
    }

    SlideshowButton = <Button> {
        text: "",
        width: 50,
        height: Fill,
        draw_bg: {
            color: #FFF0,
            color_down: #FFF2,
        },
        icon_walk: {
            width: 10
        },
        grab_key_focus: false,
    }

    SlideshowOverlay = <View> {
        cursor: Arrow,
        capture_overload: true,

        left_button = <SlideshowButton> {
            draw_icon: {
                svg_file: (LEFT_ARROW)
            }
        }
        <Filler> {}
        right_button = <SlideshowButton> {
            draw_icon: {
                svg_file: (RIGHT_ARROW)
            }
        }
    }

    Slideshow = <View> {
        <View> {
            flow: Overlay,

            image = <Image> {
                width: Fill,
                height: Fill,
                fit: Biggest,
                source: (PLACEHOLDER)
            }

            overlay = <SlideshowOverlay> {}
        }

        chat = <Chat> {
            padding: 10,
            width: 300,
            visible: false,
            draw_bg: {
                border_radius: 0.0,
                color: #fff
            }
            prompt = {
                persistent = {
                    center = {
                        left = {
                            visible: false
                        }
                        text_input = {
                            empty_text: "Ask about this image..."
                        }
                    }
                }
            }
        }
    }

    App = {{App}} {
        ui: <Root> {
            <Window> {
                body = <View> {
                    page_flip = <PageFlip> {
                        active_page: image_browser,

                        image_browser = <ImageBrowser> {}
                        slideshow = <Slideshow> {}
                    }
                }
            }
        }
        placeholder: (PLACEHOLDER)
    }
}

#[derive(Live)]
struct App {
    #[live]
    ui: WidgetRef,
    #[live]
    placeholder: LiveDependency,
    #[rust]
    state: State,
    #[rust]
    slideshow_client: Option<SlideshowClient>,
}

impl App {
    fn load_image_paths(&mut self, cx: &mut Cx, dir: &Path) {
        self.state.image_paths.clear();

        for entry in dir.read_dir().unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                self.state.image_paths.push(path);
            }
        }

        self.set_current_image(cx, 0);
    }

    fn set_current_image(&mut self, cx: &mut Cx, image_idx: usize) {
        self.state.current_image_idx = image_idx;

        let image = self.ui.image(id!(slideshow.image));
        if let Some(path) = self.state.image_paths.get(image_idx) {
            image.load_image_file_by_path_async(cx, &path).unwrap();
        } else {
            let placeholder = self.placeholder.as_str();
            image.load_image_dep_by_path(cx, placeholder).unwrap();
        }

        self.clear_slideshow_chat_messages();

        self.ui.redraw(cx);
    }

    fn go_to_previous_image(&mut self, cx: &mut Cx) {
        if self.state.current_image_idx > 0 {
            self.set_current_image(cx, self.state.current_image_idx - 1);
        }
    }

    fn go_to_next_image(&mut self, cx: &mut Cx) {
        if self.state.current_image_idx + 1 < self.state.num_images() {
            self.set_current_image(cx, self.state.current_image_idx + 1);
        }
    }

    fn configure_slideshow_chat(&mut self, cx: &mut Cx) {
        self.configure_slideshow_chat_context(cx);
        self.configure_slideshow_chat_before_hook(cx);
    }

    fn configure_slideshow_chat_context(&mut self, cx: &mut Cx) {
        let url = std::env::var("API_URL").unwrap_or_default();
        let key = std::env::var("API_KEY").unwrap_or_default();
        let mut client = OpenAIClient::new(url);
        client.set_key(&key).unwrap();

        let client = SlideshowClient::from(client);
        self.slideshow_client = Some(client.clone());

        let mut bot_context = BotContext::from(client);

        let mut chat = self.ui.chat(id!(slideshow.chat));
        chat.write().set_bot_context(cx, Some(bot_context.clone()));

        let ui = self.ui_runner();
        spawn(async move {
            let errors = bot_context.load().await.into_errors();

            ui.defer(move |me, cx, _scope| {
                let mut chat = me.ui.chat(id!(slideshow.chat));
                let mut messages = chat.read().messages_ref();

                for error in errors {
                    messages.write().messages.push(Message::app_error(error));
                }

                let model_id = std::env::var("MODEL_ID").unwrap_or_default();
                let bot = bot_context
                    .bots()
                    .into_iter()
                    .find(|b| b.id.id() == model_id);

                if let Some(bot) = bot {
                    chat.write().set_bot_id(cx, Some(bot.id));
                } else {
                    messages.write().messages.push(Message::app_error(
                        format!("Model ID '{}' not found", model_id),
                    ));
                }

                chat.write().visible = true;
                me.ui.redraw(cx);
            });
        });
    }

    fn configure_slideshow_chat_before_hook(&mut self, _cx: &mut Cx) {
        let ui = self.ui_runner();
        let mut chat = self.ui.chat(id!(slideshow.chat));
        chat.write().set_hook_before(move |task_group, _chat, _cx| {
            let before_len = task_group.len();
            task_group.retain(|task| *task != ChatTask::Send);
            if task_group.len() != before_len {
                ui.defer(move |me, cx, _scope| {
                    me.perform_chat_send(cx);
                });
            }
        });
    }

    fn perform_chat_send(&mut self, cx: &mut Cx) {
        let Some(client) = self.slideshow_client.as_mut() else {
            return;
        };

        let path =
            self.state.image_paths[self.state.current_image_idx].as_path();
        let extension = path.extension().and_then(|e| e.to_str());

        let mime = extension.map(|e| match e {
            "jpg" | "jpeg" => "image/jpeg".to_string(),
            "png" => "image/png".to_string(),
            e => format!("image/{e}"),
        });

        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or_default();

        let mut chat = self.ui.chat(id!(slideshow.chat));

        match std::fs::read(path) {
            Ok(bytes) => {
                let attachment =
                    Attachment::from_bytes(filename.to_string(), mime, &bytes);
                client.set_attachment(Some(attachment));
                chat.write().perform(cx, &[ChatTask::Send]);
            }
            Err(e) => {
                chat.read()
                    .messages_ref()
                    .write()
                    .messages
                    .push(Message::app_error(e));
            }
        }
    }

    fn clear_slideshow_chat_messages(&self) {
        self.ui
            .chat(id!(slideshow.chat))
            .read()
            .messages_ref()
            .write()
            .messages
            .retain(|m| m.from == EntityId::App);
    }
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        moly_kit::live_design(cx);
    }
}

impl LiveHook for App {
    fn after_new_from_doc(&mut self, cx: &mut Cx) {
        self.load_image_paths(cx, "../../../images".as_ref());
        self.configure_slideshow_chat(cx);
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.ui_runner()
            .handle(cx, event, &mut Scope::empty(), self);
        self.match_event(cx, event);
        let mut scope = Scope::with_data(&mut self.state);
        self.ui.handle_event(cx, event, &mut scope);
    }
}

impl MatchEvent for App {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        let page_flip = self.ui.page_flip(id!(page_flip));

        if self.ui.button(id!(button)).clicked(&actions) {
            self.clear_slideshow_chat_messages();
            page_flip.set_active_page(cx, live_id!(slideshow));
        }

        if self.ui.button(id!(left_button)).clicked(&actions) {
            self.go_to_previous_image(cx);
        }
        if self.ui.button(id!(right_button)).clicked(&actions) {
            self.go_to_next_image(cx);
        }

        if let Some(event) = self.ui.view(id!(overlay)).key_down(&actions) {
            match event.key_code {
                KeyCode::Escape => {
                    page_flip.set_active_page(cx, live_id!(image_browser))
                }
                KeyCode::ArrowLeft => self.go_to_previous_image(cx),
                KeyCode::ArrowRight => self.go_to_next_image(cx),
                _ => {}
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct ImageGrid {
    #[deref]
    view: View,
}

impl Widget for ImageGrid {
    fn draw_walk(
        &mut self,
        cx: &mut Cx2d,
        scope: &mut Scope,
        walk: Walk,
    ) -> DrawStep {
        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            let state = scope.data.get_mut::<State>().unwrap();

            if let Some(mut list) = item.as_portal_list().borrow_mut() {
                list.set_item_range(cx, 0, state.num_rows());

                while let Some(row_idx) = list.next_visible_item(cx) {
                    if row_idx >= state.num_rows() {
                        continue;
                    }

                    let row = list.item(cx, row_idx, live_id!(Row));
                    let mut scope = Scope::with_data_props(state, &row_idx);
                    row.draw_all(cx, &mut scope);
                }
            }
        }
        DrawStep::done()
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope)
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct ImageGridRow {
    #[deref]
    view: View,
}

impl Widget for ImageGridRow {
    fn draw_walk(
        &mut self,
        cx: &mut Cx2d,
        scope: &mut Scope,
        walk: Walk,
    ) -> DrawStep {
        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            let state = scope.data.get_mut::<State>().unwrap();
            let row_idx = *scope.props.get::<usize>().unwrap();

            if let Some(mut list) = item.as_portal_list().borrow_mut() {
                list.set_item_range(cx, 0, state.num_images_for_row(row_idx));

                while let Some(item_idx) = list.next_visible_item(cx) {
                    if item_idx >= state.num_images_for_row(row_idx) {
                        continue;
                    }

                    let item = list.item(cx, item_idx, live_id!(Item));

                    let image = item.image(id!(image));
                    let first_image_idx = state.first_image_for_row(row_idx);
                    let image_idx = first_image_idx + item_idx;
                    let image_path = &state.image_paths[image_idx];
                    image
                        .load_image_file_by_path_async(cx, &image_path)
                        .unwrap();

                    item.draw_all(cx, &mut Scope::empty());
                }
            }
        }
        DrawStep::done()
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope)
    }
}

struct State {
    image_paths: Vec<PathBuf>,
    max_images_per_row: usize,
    current_image_idx: usize,
}

impl State {
    fn num_images(&self) -> usize {
        self.image_paths.len()
    }

    fn num_rows(&self) -> usize {
        self.num_images().div_ceil(self.max_images_per_row)
    }

    fn first_image_for_row(&self, row_idx: usize) -> usize {
        row_idx * self.max_images_per_row
    }

    fn num_images_for_row(&self, row_idx: usize) -> usize {
        let first_image_idx = self.first_image_for_row(row_idx);
        let num_remaining_images = self.num_images() - first_image_idx;
        num_remaining_images.min(self.max_images_per_row)
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            image_paths: Vec::new(),
            max_images_per_row: 4,
            current_image_idx: 0,
        }
    }
}

app_main!(App);
