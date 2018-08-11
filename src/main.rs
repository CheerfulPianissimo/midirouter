extern crate midir;
extern crate rimd;

use midir::{MidiOutput, MidiInput};
use rimd::
{SMF,SMFWriter,SMFBuilder,Track,TrackEvent,Event,MidiMessage,AbsoluteEvent};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::io::Write;
use std::time::SystemTime;
use std::path::Path;

fn main() {
    let midi_out = MidiOutput::new("Output For MidiRouter").unwrap();
    let midi_in = MidiInput::new("Input For MidiRouter").unwrap();

    for x in 0..midi_out.port_count() {
        println!("{} {}", x, midi_out.port_name(x).unwrap());
    }
    for x in 0..midi_in.port_count() {
        println!("{} {}", x, midi_in.port_name(x).unwrap());
    }
    let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    let mut out = midi_out.connect(2, "Out").unwrap();
    let input = midi_in
        .connect(1, "In", move |stamp, message, _| {
            let mut msg_vec:Vec<u8>=vec![0;message.len()];
             for i in 0..message.len(){
                 msg_vec.insert(i,message[i]);
             }
            if msg_vec.len()!=2{
            tx.send(msg_vec);
            }
            match out.send(message){
                Result::Ok(O)=>{},
                Result::Err(E)=>{println!("Error with message {:?}",message);}
            }
        }, ())
        .unwrap();

    let mut builder=SMFBuilder::new();
    builder.add_track();
    let division=120; //120 ticks per beat
    let microsecs_per_tick = 5_000_00 / division as u64;
    let mut events_number=0;
    loop{
        let mut start_duration = SystemTime::now();
        let start_elapsed=start_duration.elapsed().unwrap();
        let start_time=(start_elapsed.as_secs() * 1_000_000) +
                    start_elapsed.subsec_micros() as u64;
        let msg=rx.recv().unwrap();

        let now_duration = start_duration.elapsed().unwrap();
        let now_time = (now_duration.as_secs() * 1_000_000) +
                    now_duration.subsec_micros() as u64;
        let delta_time=(now_time-start_time)/microsecs_per_tick;
        builder.add_event(0,TrackEvent{
            vtime:delta_time,
            event:Event::Midi(MidiMessage{data:msg})
        });
        events_number=events_number+1;

        if events_number>=50{
            break;
        }        
    }
    let mut smf=builder.result();
    smf.division=division;
    let writer=SMFWriter::from_smf(smf);    
    writer.write_to_file(&Path::new("F:\\RustProjects\\midirouter\\test.mid"));
}
