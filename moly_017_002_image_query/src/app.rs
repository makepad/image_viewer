use crate::slideshow_client::SlideshowClient;
use makepad_widgets::*;
use moly_kit::{
    ChatTask, ChatWidgetRefExt, OpenAIClient, protocol::*,
    utils::asynchronous::spawn,
};
use std::path::{Path, PathBuf};

live_design! {
    use link::widgets::*;
    use moly_kit::widgets::chat::Chat;

    PLACEHOLDER = dep("crate://self/resources/placeholder.jpg");
    LEFT_ARROW = dep("crate://self/resources/left_arrow.svg");
    RIGHT_ARROW = dep("crate://self/resources/right_arrow.svg");
    LOOKING_GLASS = dep("crate://self/resources/looking_glass.svg");

    SearchBox = <View> {
        width: 150,
        height: Fit,
        align: { y: 0.5 }
        margin: { left: 75 }

        <Icon> {
            icon_walk: { width: 12.0 }
            draw_icon: {
                color: #8,
                svg_file: (LOOKING_GLASS)
            }
        }

        query = <TextInput> {
            empty_text: "Search",
            draw_text: {
                text_style: { font_size: 10 },
                color: #8
            }
        }
    }

    MenuBar = <View> {
        width: Fill,
        height: Fit,

        <SearchBox> {}
        <Filler> {}
        slideshow_button = <Button> {
            text: "Slideshow"
        }
    }

    ImageItem = <View> {
        width: 256,
        height: 256,

        image = <Image> {
            width: Fill,
            height: Fill,
            fit: Biggest,
            source: (PLACEHOLDER)
        }
    }

    ImageRow = {{ImageRow}} {
        <PortalList> {
            height: 256,
            flow: Right,

            ImageItem = <ImageItem> {}
        }
    }

    ImageGrid = {{ImageGrid}} {
        <PortalList> {
            flow: Down,

            ImageRow = <ImageRow> {}
        }
    }

    ImageBrowser = <View> {
        flow: Down,

        <MenuBar> {}
        <ImageGrid> {}
    }

    SlideshowNavigateButton = <Button> {
        width: 50,
        height: Fill,
        draw_bg: {
            color: #fff0,
            color_down: #fff2,
        }
        icon_walk: { width: 9 },
        text: "",
        grab_key_focus: false,
    }

    SlideshowOverlay = <View> {
        height: Fill,
        width: Fill,
        cursor: Arrow,
        capture_overload: true,

        navigate_left = <SlideshowNavigateButton> {
            draw_icon: { svg_file: (LEFT_ARROW) }
        }
        <Filler> {}
        navigate_right = <SlideshowNavigateButton> {
            draw_icon: { svg_file: (RIGHT_ARROW) }
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
        placeholder: (PLACEHOLDER),

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
    }
}

#[derive(Live)]
pub struct App {
    #[live]
    placeholder: LiveDependency,
    #[live]
    ui: WidgetRef,
    #[rust]
    state: State,
    #[rust]
    slideshow_client: Option<SlideshowClient>,
}

impl App {
    fn load_image_paths(&mut self, cx: &mut Cx, path: &Path) {
        self.state.image_paths.clear();
        for entry in path.read_dir().unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            self.state.image_paths.push(path);
        }

        let query = self.ui.text_input(id!(query)).text();
        self.filter_image_paths(cx, &query);
    }

    pub fn filter_image_paths(&mut self, cx: &mut Cx, query: &str) {
        self.state.filtered_image_idxs.clear();
        for (image_idx, image_path) in self.state.image_paths.iter().enumerate()
        {
            if image_path.to_str().unwrap().contains(&query) {
                self.state.filtered_image_idxs.push(image_idx);
            }
        }
        if self.state.filtered_image_idxs.is_empty() {
            self.set_current_image(cx, None);
        } else {
            self.set_current_image(cx, Some(0));
        }
    }

    fn navigate_left(&mut self, cx: &mut Cx) {
        if let Some(image_idx) = self.state.current_image_idx {
            if image_idx > 0 {
                self.set_current_image(cx, Some(image_idx - 1));
            }
        }
    }

    fn navigate_right(&mut self, cx: &mut Cx) {
        if let Some(image_idx) = self.state.current_image_idx {
            if image_idx + 1 < self.state.num_images() {
                self.set_current_image(cx, Some(image_idx + 1));
            }
        }
    }

    fn set_current_image(&mut self, cx: &mut Cx, image_idx: Option<usize>) {
        self.state.current_image_idx = image_idx;

        let image = self.ui.image(id!(slideshow.image));
        if let Some(image_idx) = self.state.current_image_idx {
            let filtered_image_idx = self.state.filtered_image_idxs[image_idx];
            let image_path = &self.state.image_paths[filtered_image_idx];
            image
                .load_image_file_by_path_async(cx, &image_path)
                .unwrap();
        } else {
            image
                .load_image_dep_by_path(cx, self.placeholder.as_str())
                .unwrap();
        }

        self.clear_chat_messages();

        self.ui.redraw(cx);
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
        let Some(current_image_idx) = self.state.current_image_idx else {
            return;
        };

        let Some(client) = self.slideshow_client.as_mut() else {
            return;
        };

        let path = self.state.image_paths[current_image_idx].as_path();
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

    fn clear_chat_messages(&self) {
        self.ui
            .chat(id!(slideshow.chat))
            .read()
            .messages_ref()
            .write()
            .messages
            .clear();
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

impl LiveHook for App {
    fn after_new_from_doc(&mut self, cx: &mut Cx) {
        self.load_image_paths(cx, "images".as_ref());
        self.configure_slideshow_chat(cx);
    }
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        moly_kit::live_design(cx);
    }
}

impl MatchEvent for App {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if let Some(query) = self.ui.text_input(id!(query)).changed(&actions) {
            self.filter_image_paths(cx, &query);
        }

        if self.ui.button(id!(slideshow_button)).clicked(&actions) {
            self.clear_chat_messages();

            self.ui
                .page_flip(id!(page_flip))
                .set_active_page(cx, live_id!(slideshow));
        }

        if self.ui.button(id!(navigate_left)).clicked(&actions) {
            self.navigate_left(cx);
        }
        if self.ui.button(id!(navigate_right)).clicked(&actions) {
            self.navigate_right(cx);
        }

        if let Some(event) =
            self.ui.view(id!(slideshow.overlay)).key_down(&actions)
        {
            match event.key_code {
                KeyCode::Escape => self
                    .ui
                    .page_flip(id!(page_flip))
                    .set_active_page(cx, live_id!(image_browser)),
                KeyCode::ArrowLeft => self.navigate_left(cx),
                KeyCode::ArrowRight => self.navigate_right(cx),
                _ => {}
            }
        }
    }
}

#[derive(Debug)]
pub struct State {
    image_paths: Vec<PathBuf>,
    filtered_image_idxs: Vec<usize>,
    max_images_per_row: usize,
    current_image_idx: Option<usize>,
}

impl State {
    fn num_images(&self) -> usize {
        self.filtered_image_idxs.len()
    }

    fn num_rows(&self) -> usize {
        self.num_images().div_ceil(self.max_images_per_row)
    }

    fn first_image_idx_for_row(&self, row_idx: usize) -> usize {
        row_idx * self.max_images_per_row
    }

    fn num_images_for_row(&self, row_idx: usize) -> usize {
        let first_image_idx = self.first_image_idx_for_row(row_idx);
        let num_remaining_images = self.num_images() - first_image_idx;
        self.max_images_per_row.min(num_remaining_images)
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            image_paths: Vec::new(),
            filtered_image_idxs: Vec::new(),
            max_images_per_row: 4,
            current_image_idx: None,
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
            if let Some(mut list) = item.as_portal_list().borrow_mut() {
                let state = scope.data.get_mut::<State>().unwrap();

                list.set_item_range(cx, 0, state.num_rows());
                while let Some(row_idx) = list.next_visible_item(cx) {
                    if row_idx >= state.num_rows() {
                        continue;
                    }

                    let row = list.item(cx, row_idx, live_id!(ImageRow));
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
pub struct ImageRow {
    #[deref]
    view: View,
}

impl Widget for ImageRow {
    fn draw_walk(
        &mut self,
        cx: &mut Cx2d,
        scope: &mut Scope,
        walk: Walk,
    ) -> DrawStep {
        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = item.as_portal_list().borrow_mut() {
                let state = scope.data.get_mut::<State>().unwrap();
                let row_idx = *scope.props.get::<usize>().unwrap();

                list.set_item_range(cx, 0, state.num_images_for_row(row_idx));
                while let Some(item_idx) = list.next_visible_item(cx) {
                    if item_idx >= state.num_images_for_row(row_idx) {
                        continue;
                    }

                    let item = list.item(cx, item_idx, live_id!(ImageItem));
                    let image_idx =
                        state.first_image_idx_for_row(row_idx) + item_idx;
                    let filtered_image_idx =
                        state.filtered_image_idxs[image_idx];
                    let image_path = &state.image_paths[filtered_image_idx];
                    let image = item.image(id!(image));
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

app_main!(App);
