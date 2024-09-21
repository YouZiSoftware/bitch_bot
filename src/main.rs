mod deepinfra;

use std::fs;
use std::time::Duration;
use bevy_app::Update;
use bevy_ecs::event::EventReader;
use bevy_ecs::system::Res;
use kira_framework::{BotApp, kira_async, kira_recv, messages};
use kira_framework::network::connect::OneBotConnect;
use kira_framework::network::connect::reverse::OneBotReverseWebSocket;
use kira_framework::network::events::OneBotEventReceiver;
use kira_framework::network::message_chain::MessageChain;
use kira_qqbot::api::event::message::GroupMessage;
use kira_qqbot::api::event::OneBotEvents;
use kira_qqbot::connect::KiraQQBotConnect;
use kira_qqbot::{at, image, reply, text};
use std::path::PathBuf;
use std::string::ToString;
use sysinfo::System;
use kira_framework::async_manager::KiraAsyncManager;
use kira_qqbot::api::event::notice::{NotifyHonor, NotifyLuckyKing, NotifyPoke};
use kira_qqbot::messages::{At, Messages, Text};
use lazy_static::lazy_static;
use log::LevelFilter;
use crate::deepinfra::DeepInfraChat;
use pinyin::ToPinyin;
use rand_isaac::Isaac64Rng;
use rand_core::SeedableRng;
use rand::Rng;

static BOT_QQ: i64 = 3932504152;
static mut BITCH_BOT_PROMPT: Option<String> = None;
static mut MESSAGE_POKE_MANAGER: Option<Vec<i32>> = None;
lazy_static! {
    static ref MINGGAN_TEXT: Vec<&'static str> = vec![
        "bai deng qi zi xing che shuai dao"
    ];
}

fn main() {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Info)
        .init();
    DeepInfraChat::init(
        "Qwen/Qwen2-72B-Instruct",
        0.5,
        100000
    );
    unsafe {
        BITCH_BOT_PROMPT = Some(include_str!("prompt.txt").to_string());
        MESSAGE_POKE_MANAGER = Some(vec![]);
    }
    BotApp::new()
        .onebot_connect(OneBotConnect::new(
            OneBotReverseWebSocket::new(
                "127.0.0.1:8081",
                Some("shaobot"),
                Duration::from_secs(5)
            )
        ))
        .set_locale("zh-CN")
        .add_systems(Update, (receive_group_message, receive_poke, receive_honor, receive_lucky_king))
        .run::<OneBotEvents>();
}

fn receive_poke(mut receiver: EventReader<OneBotEventReceiver<NotifyPoke>>, connect: Res<OneBotConnect>) {
    kira_recv!(receiver(let event) => {
        let connect = KiraQQBotConnect::new(connect.clone());
        kira_async!("recv_event" => {
            if event.target_id == BOT_QQ {
                //random range
                let mut rng = Isaac64Rng::from_entropy();
                let random = rng.gen_range(0..=100);
                if random < 50 {
                    let yuju = vec!["你戳到我寄吧了", "想要了是吧🥵", "操死你😍", "我喜欢你❤"];
                    let rand = rng.gen_range(0..=3);
                    connect.send_group_message(
                        event.group_id,
                        messages![
                            at!(event.user_id),
                            text!("{}", yuju[rand])
                        ],
                        true
                    ).await.unwrap();
                }
            }
        });
    });
}

fn receive_honor(mut receiver: EventReader<OneBotEventReceiver<NotifyHonor>>, connect: Res<OneBotConnect>) {
    kira_recv!(receiver(let event) => {
        let connect = KiraQQBotConnect::new(connect.clone());
        kira_async!("recv_event" => {
            if event.honor_type == "talkative".to_string() {
                connect.send_group_message(
                        event.group_id,
                        messages![
                            at!(event.user_id),
                            text!("🐉王喷个水💧看看👀")
                        ],
                        true
                    ).await.unwrap();
            }
        });
    });
}

fn receive_lucky_king(mut receiver: EventReader<OneBotEventReceiver<NotifyLuckyKing>>, connect: Res<OneBotConnect>) {
    kira_recv!(receiver(let event) => {
        let connect = KiraQQBotConnect::new(connect.clone());
        kira_async!("recv_event" => {
            connect.send_group_message(
                        event.group_id,
                        messages![
                            at!(event.target_id),
                            text!("vivo50看看实力👀")
                        ],
                        true
                    ).await.unwrap();
        });
    });
}

fn receive_group_message(mut receiver: EventReader<OneBotEventReceiver<GroupMessage>>, connect: Res<OneBotConnect>) {
    kira_recv!(receiver(let event) => {
        let connect = KiraQQBotConnect::new(connect.clone());
        kira_async!("recv_event" => {
            check_mingan(connect.clone(), event.clone()).await;
            if let Some(at) = event.message.get::<At>(0) {
                let mut message = event.message.clone();
                message.remove::<At>(0);
                if at.qq == BOT_QQ.to_string() {
                    process_group_message(connect.clone(), message, event).await;
                    return;
                }
            }
            if event.message.start_with::<Text>() {
                let message = event.message.clone();
                let message = message.as_persistent_string::<Messages>();
                if message.starts_with("/") {
                    process_command(connect.clone(), message, event).await;
                }
            }
        });
    });
}

async fn check_mingan(connect: KiraQQBotConnect, event: GroupMessage) {
    let message = event.raw_message;
    let mut pinyin = "".to_string();
    for py in message.as_str().to_pinyin() {
        if let Some(py) = py {
            pinyin += py.plain();
            pinyin += " ";
        }
    }
    let pinyin = pinyin.trim();
    for mingan in MINGGAN_TEXT.iter() {
        if pinyin.contains(mingan) {
            let _ = connect.send_group_message(
                event.group_id,
                messages![
                    text!("检测到政治因素")
                ],
                false
            ).await;
        }
        return;
    }
}

async fn process_group_message(connect: KiraQQBotConnect, message: MessageChain, event: GroupMessage) {
    let message = message.as_persistent_string::<Messages>();
    let tips = connect.send_group_message(
        event.group_id,
        messages![
            text!("别吵, 我在思考... 戳一戳我取消思考")
        ],
        false
    ).await.unwrap();

    let event_clone = event.clone();
    let connect_clone = connect.clone();

    let send_msg = tokio::spawn(async move {
        unsafe {
            MESSAGE_POKE_MANAGER.as_mut().unwrap().push(event_clone.message_id);
        }
        let content = DeepInfraChat::global().get(event_clone.group_id);
        let result = content.chat(
            unsafe { BITCH_BOT_PROMPT.clone().unwrap() },
            Some(format!("NAME: {}(QQ_ID: {})", event_clone.sender.nickname.unwrap(), event_clone.sender.user_id.unwrap())),
            message.as_str()
        ).await;
        let _ = connect_clone.recall_message(tips).await;
        if let Ok(msg) = result {
            connect_clone.send_group_message(
                event_clone.group_id,
                messages![
                    reply!(event_clone.message_id),
                    at!(event_clone.sender.user_id.unwrap()),
                    text!("{}", msg)
                ],
                true
            ).await.unwrap();
        }else {
            let error = result.err().unwrap();
            connect_clone.send_group_message(
                event_clone.group_id,
                messages![
                    reply!(event_clone.message_id),
                    text!("我错了跌: \n{}", error)
                ],
                true
            ).await.unwrap();
        }
        unsafe {
            MESSAGE_POKE_MANAGER.as_mut().unwrap().retain(|x| {
                x != &event_clone.message_id
            })
        }
    });

    let event_clone = event.clone();
    let connect_clone = connect.clone();
    let _ = tokio::spawn(async move {
        while let Ok(poke) = connect_clone.wait_event::<NotifyPoke>().await {
            if poke.group_id == event_clone.group_id && poke.user_id == event_clone.sender.user_id.unwrap() && poke.target_id == BOT_QQ {
                unsafe {
                    if !MESSAGE_POKE_MANAGER.as_ref().unwrap().contains(&event_clone.message_id) {
                        return;
                    }
                }
                send_msg.abort();
                connect_clone.recall_message(tips).await.unwrap();
                connect_clone.send_group_message(
                    event_clone.group_id,
                    messages![
                        reply!(event_clone.message_id),
                        text!("消息撤回成功")
                    ],
                    true
                ).await.unwrap();
                return;
            }
        }
    });
}

async fn process_command(connect: KiraQQBotConnect, message: String, event: GroupMessage) {
    let message = &message[1..];
    if message == "help" || message == "帮助" {
        connect.send_group_message(
            event.group_id,
            messages!(image!(file("/root/lagrange/help.png"))),
            false
        ).await.unwrap();
    }else if message == "prompt_list" || message == "提示词列表" {
        connect.send_group_message(
            event.group_id,
            messages!(image!(file("/root/lagrange/prompt_list.png"))),
            false
        ).await.unwrap();
    }else if message == "usage" || message == "查看占用" {
        let mut sys = System::new_all();
        sys.refresh_all();

        let memory_usage = sys.used_memory() as f32 / sys.total_memory() as f32 * 100.0;

        let total_memory_gb = sys.total_memory() as f32 / 1024.0 / 1024.0 / 1024.0;
        let swap_usage = (sys.used_swap() as f32 / sys.total_swap() as f32) * 100.0;
        let total_swap_gb = sys.total_swap() as f32 / 1024.0 / 1024.0 / 1024.0;

        connect.send_group_message(
            event.group_id,
            messages![
                text!("🖥烧b0t 3.0 - 占用信息🖥:\n"),
                text!("系统: {} {}\n", System::name().unwrap(), System::os_version().unwrap()),
                text!("内存占用: {:.2}%(总共: {:.2} GB)\n", memory_usage, total_memory_gb),
                text!("swap占用: {:.2}%(总共: {:.2} GB)", swap_usage, total_swap_gb)
            ],
            false
        ).await.unwrap();
    }else if message == "about" || message == "关于" {
        connect.send_group_message(
            event.group_id,
            messages![
                text!("🔥烧b0t 3.0 - 关于🔥:\n"),
                text!("🦀Rust版本🦀: 1.82.0-nightly\n"),
                text!("👨作者QQ👨: 1069743308\n"),
                text!("架构: KiraFramework 0.2.0(github: https://github.com/YouZiSoftware/KiraFramework)")
            ],
            false
        ).await.unwrap();
    }else if message == "clear" || message == "清除对话" {
        let content = DeepInfraChat::global().get(event.group_id);
        content.clear().await;
        connect.send_group_message(
            event.group_id,
            messages![
                text!("清空对话成功, 但是你的犯罪证据已被我上传到晶格内网👁👁")
            ],
            false
        ).await.unwrap();
    }else if message == "reload_prompt" || message == "重载提示词" {
        if event.sender.user_id.unwrap() == 1069743308 {
            unsafe {
                let content = fs::read("/root/lagrange/prompt.txt").unwrap();
                *BITCH_BOT_PROMPT.as_mut().unwrap() = String::from_utf8(content).unwrap();
                connect.send_group_message(
                    event.group_id,
                    messages![
                        text!("重载提示词成功")
                    ],
                    false
                ).await.unwrap();
            }
        }else {
            connect.send_group_message(
                event.group_id,
                messages![
                    text!("你没有权限执行此命令")
                ],
                false
            ).await.unwrap();
        }
    }
}