extern crate cursive;
extern crate midir;
extern crate rimd;

use cursive::traits::{Boxable, Identifiable, Scrollable};
use cursive::views::*;
use cursive::Cursive;
use midir::{MidiInput, MidiOutput};
use rimd::{Event, MidiMessage, SMFBuilder, SMFWriter, TrackEvent};
use std::fs::read_dir;
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::spawn;
use std::time::{Duration, SystemTime};

fn main() {
    setup();
}

fn start(out_port: usize, in_port: usize, path: &str) {
    let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    let midi_out = MidiOutput::new("Output For MidiRouter").unwrap();
    let midi_in = MidiInput::new("Input For MidiRouter").unwrap();

    let mut out = midi_out.connect(out_port, "Out").unwrap();
    let _input = midi_in
        .connect(
            in_port,
            "In",
            move |_stamp, message, _record_sender| {
                let mut msg_vec: Vec<u8> = vec![0; message.len()];
                for i in 0..message.len() {
                    msg_vec.insert(i, message[i]);
                }
                if msg_vec.len() != 2 {
                    tx.send(msg_vec).unwrap();
                }
                match out.send(message) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error with message {:?}  {:?}", e, message);
                    }
                }
            },
            (),
        )
        .unwrap();

    record(rx, path);
}

fn record(rx: Receiver<Vec<u8>>, path: &str) {
    let mut builder = SMFBuilder::new();
    builder.add_track();
    let division = 120; //120 ticks per beat
    let microsecs_per_tick = 5_000_00 / division as u64;
    let mut events_number = 0;
    loop {
        let start_duration = SystemTime::now();
        let msg = rx.recv().unwrap();

        let delta_time = duration_to_micros(&start_duration.elapsed().unwrap());
        let delta_ticks = (delta_time) / microsecs_per_tick;

        builder.add_event(
            0,
            TrackEvent {
                vtime: delta_ticks,
                event: Event::Midi(MidiMessage { data: msg }),
            },
        );
        events_number = events_number + 1;

        if events_number%1000==0 {
            let mut smf = builder.clone().result();
            smf.division = division;
            let writer = SMFWriter::from_smf(smf);
            writer.write_to_file(&Path::new(&path)).unwrap();
        }
    }
}

fn duration_to_micros(duration: &Duration) -> u64 {
    (duration.as_secs() * 1_000_000) + duration.subsec_micros() as u64
}

fn setup() {
    let mut siv = Cursive::default();

    let midi_out = MidiOutput::new("Output For MidiRouter").unwrap();
    let midi_in = MidiInput::new("Input For MidiRouter").unwrap();

    let mut out_box = LinearLayout::vertical();
    let mut out_group: RadioGroup<usize> = RadioGroup::new();
    for x in 0..midi_out.port_count() {
        let button = out_group.button(x, midi_out.port_name(x).unwrap());
        out_box.add_child(button);
    }

    let mut in_box = LinearLayout::vertical();
    let mut in_group: RadioGroup<usize> = RadioGroup::new();
    for x in 0..midi_in.port_count() {
        let button = in_group.button(x, midi_in.port_name(x).unwrap());
        in_box.add_child(button);
    }
    let mut hbox = LinearLayout::horizontal();
    hbox.add_child(Dialog::around(in_box.scrollable().fixed_size((30, 10))).title("MIDI Input"));
    hbox.add_child(Dialog::around(out_box.scrollable().fixed_size((30, 10))).title("MIDI Output"));

    let file_text = TextArea::new()
        .disabled()
        .with_id("path")
        .fixed_size((60,1)).scrollable();
    let filebox = LinearLayout::horizontal().child(file_text);

    let mut vbox = LinearLayout::vertical().child(hbox);
    vbox.add_child(Dialog::around(filebox).title("Log"));
    vbox.add_child(Button::new("Start", move |s| {
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
        let path = format!("{}.mid", max + 1);
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
        spawn(move || start(out_port, in_port,&path));
    }));
    siv.add_layer(Dialog::around(vbox));
    siv.add_global_callback('q', |s| s.quit());

    siv.run();
}
