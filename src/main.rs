#![windows_subsystem = "windows"]
#![allow(unused)]
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::{env, thread};
use std::time::Duration;
use battery;
use battery::units::power::watt;
use tray_icon::{
    menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder, TrayIconEvent,
};
use winit::event_loop::{ControlFlow, EventLoopBuilder};

mod batinfo;

fn update_tooltip(tray_icon: &Arc<Mutex<Option<tray_icon::TrayIcon>>>, new_tooltip: &str) {
    if let Some(icon) = tray_icon.lock().unwrap().as_ref() {
        icon.set_tooltip(Some(new_tooltip));
    }
}

fn main() {
    //icon must be distributed alongside bin until I find a half decent way to pack it
    //<a target="_blank" href="https://icons8.com/icon/77340/battery">Battery</a> icon by <a target="_blank" href="https://icons8.com">Icons8</a>
    let path = concat!("./icons/icon.png");
    println!("{}", path);

    // for linux in the future maybe
    #[cfg(target_os = "linux")]
    std::thread::spawn(|| {
        use tray_icon::menu::Menu;

        let icon = load_icon(std::path::Path::new(path));

        gtk::init().unwrap();
        let _tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(Menu::new()))
            .with_icon(icon)
            .build()
            .unwrap();

        gtk::main();
    });

    let event_loop = EventLoopBuilder::new().build().unwrap();

    #[cfg(not(target_os = "linux"))]
        let tray_icon = Arc::new(Mutex::new(None));

    let (tx, rx): (Sender<String>, Receiver<String>) = channel();

    let update_tooltip_with_battery_info = {
        let tx_clone = tx.clone(); // Clone the sender part of the channel
        move || {
            match batinfo::get_battery_info() {
                Ok(Some(battery)) => {
                    let charge = battery.energy_rate().get::<watt>();
                    let tooltip_text = format!("Battery: {} W", charge);
                    tx_clone.send(tooltip_text).unwrap();
                },
                Ok(None) => tx_clone.send("No battery found".to_string()).unwrap(),
                Err(e) => eprintln!("Failed to get battery info: {}", e),
            }
        }
    };

    thread::spawn(move || {
        loop {
            update_tooltip_with_battery_info();
            thread::sleep(Duration::from_secs(1)); // Update every second (if this is removed it will follow the behavior on line 83/87
        }
    });

    let menu_channel = MenuEvent::receiver();
    let tray_channel = TrayIconEvent::receiver();

    event_loop.run(move |event, event_loop| {
        // We add delay of 16 ms (60fps) to en be removed to allow ControlFlow::Poll to poll on each cpu cyclevent_loop to reduce cpu load. (increased to 64 or 15fps)
        // if this is removed and no other limits are placed like on line 70, it will update every cpu cycle
        // see https://github.com/tauri-apps/tray-icon/issues/83#issuecomment-1697773065
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(64),
        ));

        #[cfg(not(target_os = "linux"))]
        if let winit::event::Event::NewEvents(winit::event::StartCause::Init) = event {
            let icon = load_icon(std::path::Path::new(path));

            let tray_icon_instance = TrayIconBuilder::new()
                .with_menu(Box::new(Menu::new()))
                .with_tooltip("winit - awesome windowing lib")
                .with_icon(icon)
                .with_title("x")
                .build()
                .unwrap();

            *tray_icon.lock().unwrap() = Some(tray_icon_instance);
        }

        if let Ok(new_tooltip) = rx.try_recv() {
            if let Some(icon) = tray_icon.lock().unwrap().as_ref() {
                icon.set_tooltip(Some(new_tooltip));
            }
        }

        if let Ok(event) = tray_channel.try_recv() {
            println!("{event:?}");
        }
    });
}

fn load_icon(path: &std::path::Path) -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}