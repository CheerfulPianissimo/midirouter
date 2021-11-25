extern crate crossbeam_channel;
extern crate cursive;
extern crate midir;
extern crate rimd;

use channels::{Receiver, Sender};
use crossbeam_channel as channels;
use cursive::traits::Scrollable;
use cursive::traits::{Boxable, Identifiable};
use cursive::views::*;
use cursive::Cursive;
use cursive::traits::View;
use cursive::event::Key;
use midir::{MidiInput, MidiOutput};
use rimd::{Event, MidiMessage, SMFBuilder, SMFWriter, TrackEvent};
use std::fs::read_dir;
use std::path::Path;
use std::thread::spawn;
use std::time::{Duration, SystemTime};

fn main() {
    setup();
}

fn start(out_port: usize, in_port: usize, path: &str, recv: Receiver<bool>) {
    let (tx, rx): (Sender<Option<Vec<u8>>>, Receiver<Option<Vec<u8>>>) = channels::unbounded();
    let midi_out = MidiOutput::new("Output For MidiRouter").unwrap();
    let midi_in = MidiInput::new("Input For MidiRouter").unwrap();
    let mut out = midi_out.connect(out_port, "Out").unwrap();
    let input = midi_in
        .connect(
            in_port,
            "In",
            move |_stamp, message, _record_sender| {
                if recv.try_recv()==Ok(false){
                    tx.send(None).unwrap();
                    return;
                }
                let mut msg_vec: Vec<u8> = vec![0; message.len()];
                for i in 0..message.len() {
                    msg_vec.insert(i, message[i]);
                }

                if msg_vec.len() != 2 {
                    tx.send(Some(msg_vec)).unwrap();
                }
                match out.send(message) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error with message {:?}  {:?}", e, message);
                    }
                }
            },
            (),
        ).unwrap();

    record(rx, path);
    input.close();
}

fn record(rx: Receiver<Option<Vec<u8>>>, path: &str) {
    let division = 120; //120 ticks per beat
    let microsecs_per_tick = 5_000_00 / division as u64;
    let mut events_number = 0;
    let mut events : Vec<(u64,Event)>=Vec::new();

    loop {
        let start_duration = SystemTime::now();
        let msg = match rx.recv().unwrap(){
            Some(message)=>message,
            None=>break //Message to stop recording
        };

        let delta_time = duration_to_micros(&start_duration.elapsed().unwrap());
        let delta_ticks = (delta_time) / microsecs_per_tick;

        events.push((delta_ticks,Event::Midi(MidiMessage { data: msg })));
        events_number += 1;

        if events_number % 100 == 0 { //Autosave
            output_events_to_file(path,&events,division)
        }
    }
    output_events_to_file(path,&events,division)
}

fn output_events_to_file(path: &str,events : &Vec<(u64,Event)>,division : i16){
    let mut smf_builder  = SMFBuilder::new();
    smf_builder.add_track();
    for (tick, event) in events{
        let event_clone=event.clone();
        smf_builder.add_event(
            0,
            TrackEvent {
                vtime: *tick,
                event: event_clone,
            },
        );
    }
    let mut smf=smf_builder.result();
    smf.division = division;
    let writer = SMFWriter::from_smf(smf);
    writer.write_to_file(&Path::new(&path)).unwrap();
}

fn duration_to_micros(duration: &Duration) -> u64 {
    (duration.as_secs() * 1_000_000) + duration.subsec_micros() as u64
}

fn setup() {
    let mut siv = Cursive::default();

    let midi_out = MidiOutput::new("Output For MidiRouter").unwrap();
    let midi_in = MidiInput::new("Input For MidiRouter").unwrap();

    let args: Vec<String> = std::env::args().collect();
    let mut arg_ports=None;
    if args.len()==3 {
        match args[1].parse::<usize>() {
            Ok(in_port)=>{
                match args[2].parse::<usize>() {
                    Ok(out_port)=>{
                        arg_ports=Some((in_port,out_port));
                    },
                    Err(_)=>{}
                }
            },
            Err(_)=>{}
        }
    } 

    let mut out_box = LinearLayout::vertical();
    let mut out_group: RadioGroup<usize> = RadioGroup::new();
    for x in 0..midi_out.port_count() {
        let mut button = out_group.button(x, midi_out.port_name(x).unwrap());
        if let Some((_in_port,out_port))=arg_ports{
            if out_port==x{
                button.select();
            }
        }
        out_box.add_child(button);
    }

    let mut in_box = LinearLayout::vertical();
    let mut in_group: RadioGroup<usize> = RadioGroup::new();
    for x in 0..midi_in.port_count() {
        let mut button = in_group.button(x, midi_in.port_name(x).unwrap());
        if let Some((in_port,_out_port))=arg_ports{
            if in_port==x{
                button.select();
            }
        }
        in_box.add_child(button);
    }


    let mut hbox = LinearLayout::horizontal();
    hbox.add_child(Dialog::around(in_box.fixed_size((30, 10))).title("MIDI Input"));
    hbox.add_child(Dialog::around(out_box.fixed_size((30, 10))).title("MIDI Output"));

    let file_text = TextArea::new()
        .disabled()
        .with_id("path")
        .fixed_size((60, 3));
    let filebox = LinearLayout::horizontal().child(file_text);

    let (send, recv) = channels::bounded::<bool>(1);
    let recv_clone = recv.clone();

    let mut vbox = LinearLayout::vertical().child(hbox);
    vbox.add_child(Dialog::around(filebox).title("Log"));
    vbox.add_child(Button::new("Start", move |s| {
        let path = get_out_path();
        let out_port = out_group.selected_id();
        let in_port = in_group.selected_id();
        s.call_on_id("path", |view: &mut TextArea| {
            view.set_content(format!(
                "Joining {} with {}, recording to {}",
                midi_in.port_name(in_port).unwrap(),
                midi_out.port_name(out_port).unwrap(),
                path
            ));
        }).unwrap();
        let recv_clone2=recv_clone.clone();
        spawn(move || start(out_port, in_port, &path, recv_clone2));
    }).with_id("start-button"));
    vbox.add_child(Button::new("Stop Most Recent Connection",
                               move |s| {
        if send.is_empty() {
            send.send(false).unwrap();  //To redirector thread
        }
        s.call_on_id("path", |view: &mut TextArea| {
            view.set_content(format!("Stopping Most Recent Connection"));
        }).unwrap();
    }).with_id("stop-button"));
    vbox.add_child(Button::new("Reset Most Recent Connection", move |s| {
        s.call_on_id("stop-button", |view: &mut Button| {
            view.on_event(cursive::event::Event::Key(Key::Enter));
        }).unwrap();
        s.call_on_id("start-button", |view: &mut Button| {
            view.on_event(cursive::event::Event::Key(Key::Enter));
        }).unwrap();
    }));
    siv.add_layer(Dialog::around(vbox.scrollable()));
    siv.add_global_callback('q', |s| s.quit());

    match arg_ports{
        Some((in_port,out_port))=>{
            let path = get_out_path();
            siv.call_on_id("path", |view: &mut TextArea| {
                view.set_content(format!(
                    "Joining {} with {}, recording to {}",
                    in_port,out_port,path
                ));
            }).unwrap();
            let recv_clone2 = recv.clone();
            spawn(move || start(out_port, in_port, &path, recv_clone2));
        },
        None=>{}
    }

    siv.run();
}


fn get_out_path()->String{
    let mut max = 0;
    for i in read_dir(Path::new("./")).unwrap() {
        let path = i.unwrap().path();
        let extension = match path.extension() {
            Some(e) => e,
            None => {
                continue;
            }
        };
        if extension.eq("mid") {
            let file_stem = path.file_stem().unwrap().to_str().unwrap();
            match file_stem.parse::<u32>() {
                Ok(number) => {
                    if number > max {
                        max = number;
                    }
                }
                Err(_) => continue,
            };
        }
    }
    format!("{}.mid", max + 1)
}