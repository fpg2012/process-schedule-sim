mod task;
mod scheduler;
mod memory;
mod processor;
mod manager;
mod condition;

use std::rc::Rc;
use std::cell::RefCell;
use crate::manager::*;
use gtk::prelude::BuilderExtManual;
use gtk::{WidgetExt, Inhibit, ButtonExt, Application, GtkWindowExt, EntryExt, LabelExt};
use gio::ApplicationExt;
use gio::prelude::ApplicationExtManual;

fn main() {
    let mut manager = Manager::new();
    manager.create_task(3, 1, 400, None);
    manager.create_task(2, 1, 200, None);
    manager.create_task(4, 1, 50, None);
    manager.create_task(3, 1, 1200, Some(3));
    manager.create_task(5, 1, 1200, None);
    manager.create_task(2, 1, 2000, None);
    manager.create_task(6, 1, 20, None);

    let manager = Rc::new(RefCell::new(manager));

    let application = Application::new(
        Some("com.github.fpg2012.os_exp1"),
        Default::default(),
    ).expect("failed to initialize GTK application");

    application.connect_activate(move |app| {
        let glade_src = include_str!("resource.glade");
        let builder = gtk::Builder::from_string(glade_src);
        let window: gtk::Window = builder.get_object("window").unwrap();
        window.set_application(Some(app));

        let draw_area: gtk::DrawingArea = builder.get_object("draw_area").unwrap();
        let exec_button: gtk::Button = builder.get_object("exec_button").unwrap();
        let next_button: gtk::Button = builder.get_object("next_button").unwrap();
        let entry: gtk::Entry = builder.get_object("command_entry").unwrap();
        let msg_label: gtk::Label = builder.get_object("msg_label").unwrap();
        let time_label: gtk::Label = builder.get_object("time_label").unwrap();

        let draw_area_copy = draw_area.clone();
        let entry_clone = entry.clone();
        let manager_clone = manager.clone();
        exec_button.connect_clicked(move |_| {
            // get text in entry
            let cmd = String::from(entry_clone.get_text().to_string().trim());
            println!("exec command {}", cmd.as_str());
            // parse
            let (mut req_time, mut priority, mut mem_size, mut pre): (i32, u32, u32, Option<u32>) = (0, 0, 0, None);
            let temp: Vec<&str> = cmd.split(char::is_whitespace).collect();
            if temp.len() != 3 && temp.len() != 4 {
                eprintln!("[Error] invalid command");
                msg_label.set_text("[Error] invalid command");
                return;
            }
            req_time = temp[0].parse().unwrap();
            priority = temp[1].parse().unwrap();
            mem_size = temp[2].parse().unwrap();
            if temp.len() == 4 {
                pre = Some(temp[3].parse().unwrap());
            }
            manager_clone.borrow_mut().create_task(req_time, priority, mem_size, pre);
            // clear text
            entry_clone.set_text("");
            msg_label.set_text("[Ok] command executed.")
        });

        let draw_area_copy = draw_area.clone();
        let manager_clone = manager.clone();
        next_button.connect_clicked(move |_| {
            manager_clone.borrow_mut().advance();
            draw_area_copy.queue_draw();
        });

        let manager_clone = manager.clone();
        draw_area.connect_draw(move |_, cr| {
            // make tasks colorful
            let colors = [
                (0.51, 0.67, 0.87, 0.8),
                (0.51, 0.87, 0.67, 0.8),
                (0.67, 0.51, 0.87, 0.8),
                (0.67, 0.87, 0.51, 0.8),
                (0.87, 0.67, 0.51, 0.8),
                (0.87, 0.51, 0.67, 0.8),
            ];
            let set_pid_color = |pid: &u32| {
                let (r, g, b, a) = colors[((pid - 1) % colors.len() as u32) as usize];
                cr.set_source_rgba(r, g, b, a);
            };
            let set_text_color = || {
                cr.set_source_rgba(0.3, 0.3, 0.3, 1.0);
            };
            // set font
            // draw time
            // cr.move_to(250.0, 22.0);
            // cr.set_font_size(18.0);
            // cr.show_text(format!("Time: {}", manager_clone.borrow().time()).as_str());
            time_label.set_text(format!("Time: {}", manager_clone.borrow().time()).as_str());
            // draw memory
            let (bx, by, w, h) = (30.0, 30.0, 140.0, 340.0);
            set_text_color();
            cr.set_line_width(1.0);
            cr.rectangle(bx, by, w, h);
            cr.stroke();

            let cth = |mem_pos: u32| {
                (mem_pos as f64) / (4096.0) * (h as f64)
            };

            let (pw, ph) = (80.0, 80.0);
            // draw processors
            let proc1_rect = || {
                let (px1, py1) = (260.0, 60.0);
                cr.rectangle(px1, py1, pw, ph);
            };
            let proc2_rect = || {
                let (px2, py2) = (260.0, 260.0);
                cr.rectangle(px2, py2, pw, ph);
            };
            proc1_rect();
            cr.stroke();
            proc2_rect();
            cr.stroke();

            // fill mem
            // set font size
            cr.set_font_size(13.0);
            for (pid, hole) in manager_clone.borrow().get_mem_usage().iter() {
                let (beg, _) = hole.to_tuple();
                set_pid_color(pid);
                cr.rectangle(bx, cth(beg) + by, w, cth(hole.get_size()));
                cr.fill();
                cr.set_source_rgba(0.3, 0.3, 0.3, 1.0);
                cr.rectangle(bx, cth(beg) + by, w, cth(hole.get_size()));
                cr.stroke();
                cr.move_to(bx + w + 5.0, cth(beg) + cth(hole.get_size()) / 2.0 + by);
                cr.show_text(format!("pid: {}", pid).as_str());
            }

            // add text to proc
            // set font size
            cr.set_font_size(16.0);
            let running = manager_clone.borrow().get_running_task();
            let (r1, r2) = (running[0], running[1]);
            if let Some(r) = r1 {
                set_pid_color(&r);
                proc1_rect();
                cr.fill();
                set_text_color();
                cr.move_to(300.0 - 20.0, 100.0);
                cr.show_text(format!("pid {}", r).as_str());
            } else {
                set_text_color();
                cr.move_to(300.0 - 20.0, 100.0);
                cr.show_text("idle");
            }
            if let Some(r) = r2 {
                set_pid_color(&r);
                proc2_rect();
                cr.fill();
                set_text_color();
                cr.move_to(300.0 - 20.0, 300.0);
                cr.show_text(format!("pid {}", r).as_str());
            } else {
                set_text_color();
                cr.move_to(300.0 - 20.0, 300.0);
                cr.show_text("idle");
            }
            Inhibit(false)
        });

        window.show_all();
    });

    application.run(&[]);
}
