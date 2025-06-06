use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

use crate::services::event_bus::EventBus;
use crate::{services::websocket::WebsocketService, User};

pub enum Msg {
    HandleMsg(String),
    SubmitMessage,
}

#[derive(Deserialize, Debug, Clone)]
struct MessageData {
    from: String,
    message: String,
    #[serde(default)]
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MsgTypes {
    Users,
    Register,
    Message,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct WebSocketMessage {
    message_type: MsgTypes,
    data_array: Option<Vec<String>>,
    data: Option<String>,
}

#[derive(Clone, Debug)]
struct UserProfile {
    name: String,
    avatar: String,
}

pub struct Chat {
    users: Vec<UserProfile>,
    chat_input: NodeRef,
    _producer: Box<dyn Bridge<EventBus>>,
    wss: WebsocketService,
    messages: Vec<MessageData>,
}

impl Component for Chat {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let (user, _) = ctx
            .link()
            .context::<User>(Callback::noop())
            .expect("context to be set");
        let wss = WebsocketService::new();
        let username = user.username.borrow().clone();

        let message = WebSocketMessage {
            message_type: MsgTypes::Register,
            data: Some(username.to_string()),
            data_array: None,
        };

        if let Ok(_) = wss
            .tx
            .clone()
            .try_send(serde_json::to_string(&message).unwrap())
        {
            log::debug!("message sent successfully");
        }

        Self {
            users: vec![],
            messages: vec![],
            chat_input: NodeRef::default(),
            wss,
            _producer: EventBus::bridge(ctx.link().callback(Msg::HandleMsg)),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::HandleMsg(s) => {
                match serde_json::from_str::<WebSocketMessage>(&s) {
                    Ok(msg) => {
                        match msg.message_type {
                            MsgTypes::Users => {
                                let users_from_message = msg.data_array.unwrap_or_default();
                                self.users = users_from_message
                                    .iter()
                                    .map(|u| UserProfile {
                                        name: u.into(),
                                        avatar: format!(
                                            "https://avatars.dicebear.com/api/adventurer-neutral/{}.svg",
                                            u
                                        )
                                        .into(),
                                    })
                                    .collect();
                                return true;
                            }
                            MsgTypes::Message => {
                                if let Some(data) = msg.data {
                                    match serde_json::from_str::<MessageData>(&data) {
                                        Ok(message_data) => {
                                            log::debug!("Received message: {:?}", message_data);
                                            self.messages.push(message_data);
                                            return true;
                                        }
                                        Err(e) => {
                                            log::error!("Error parsing message data: {:?}", e);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        log::error!("Error parsing websocket message: {:?}", e);
                    }
                }
                false
            }
            Msg::SubmitMessage => {
                let input = self.chat_input.cast::<HtmlInputElement>();
                if let Some(input) = input {
                    let js_time = js_sys::Date::new_0().to_locale_time_string("id-ID");
                    let current_time = js_time.as_string().unwrap_or_default();
                    
                    let message_content = input.value();
                    if !message_content.is_empty() {
                        let message_data = serde_json::json!({
                            "message": message_content,
                            "timestamp": current_time
                        });
                        
                        let message = WebSocketMessage {
                            message_type: MsgTypes::Message,
                            data: Some(serde_json::to_string(&message_data).unwrap()),
                            data_array: None,
                        };
                        
                        if let Err(e) = self
                            .wss
                            .tx
                            .clone()
                            .try_send(serde_json::to_string(&message).unwrap())
                        {
                            log::error!("Error sending to channel: {:?}", e);
                        }
                        input.set_value("");
                    }
                };
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let submit = ctx.link().callback(|_| Msg::SubmitMessage);

        html! {
            <div class="flex w-screen">
                <div class="flex-none w-56 h-screen bg-gray-100">
                    <div class="text-xl p-3">{"Users"}</div>
                    {
                        self.users.clone().iter().map(|u| {
                            html!{
                                <div class="flex m-3 bg-white rounded-lg p-2">
                                    <div>
                                        <img class="w-12 h-12 rounded-full" src={u.avatar.clone()} alt="avatar"/>
                                    </div>
                                    <div class="flex-grow p-3">
                                        <div class="flex text-xs justify-between">
                                            <div>{u.name.clone()}</div>
                                        </div>
                                        <div class="text-xs text-gray-400">
                                            {"Hi there!"}
                                        </div>
                                    </div>
                                </div>
                            }
                        }).collect::<Html>()
                    }
                </div>
                <div class="grow h-screen flex flex-col">
                    <div class="w-full h-14 border-b-2 border-gray-300"><div class="text-xl p-3">{"💬 Chat!"}</div></div>
                    <div class="w-full grow overflow-auto border-b-2 border-gray-300">
                        {
                            self.messages.iter().map(|m| {
                                // Find user or create a default one if not found
                                let user_opt = self.users.iter().find(|u| u.name == m.from);
                                let user = match user_opt {
                                    Some(u) => u.clone(),
                                    None => UserProfile {
                                        name: m.from.clone(),
                                        avatar: format!(
                                            "https://avatars.dicebear.com/api/adventurer-neutral/{}.svg",
                                            m.from
                                        ),
                                    }
                                };
                                
                                html!{
                                    <div class="flex items-end w-3/6 bg-gray-100 m-8 rounded-tl-lg rounded-tr-lg rounded-br-lg ">
                                        <img class="w-8 h-8 rounded-full m-3" src={user.avatar.clone()} alt="avatar"/>
                                        <div class="p-3">
                                            <div class="flex justify-between items-center">
                                                <div class="text-sm">
                                                    {m.from.clone()}
                                                </div>
                                                <div class="text-xs text-gray-400 ml-2">
                                                    {m.timestamp.clone().unwrap_or_default()}
                                                </div>
                                            </div>
                                            <div class="text-xs text-gray-500 mt-1">
                                                {
                                                    if m.message.ends_with(".gif") {
                                                        html! {
                                                            <img class="mt-3" src={m.message.clone()}/>
                                                        }
                                                    } else {
                                                        html! { {m.message.clone()} }
                                                    }
                                                }
                                            </div>
                                        </div>
                                    </div>
                                }
                            }).collect::<Html>()
                        }
                    </div>
                    <div class="w-full h-14 flex px-3 items-center">
                        <input ref={self.chat_input.clone()} type="text" placeholder="Message" class="block w-full py-2 pl-4 mx-3 bg-gray-100 rounded-full outline-none focus:text-gray-700" name="message" required=true />
                        <button onclick={submit} class="p-3 shadow-sm bg-blue-600 w-10 h-10 rounded-full flex justify-center items-center color-white">
                            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" class="fill-white">
                                <path d="M0 0h24v24H0z" fill="none"></path><path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"></path>
                            </svg>
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}