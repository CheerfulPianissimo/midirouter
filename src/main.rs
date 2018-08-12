extern crate midir;
extern crate rimd;
extern crate cursive;

use midir::{MidiOutput, MidiInput};
use rimd::{SMFWriter,SMFBuilder,TrackEvent,Event,MidiMessage};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::time::{SystemTime,Duration};
use std::path::Path;
use std::thread::{spawn};
use cursive::{Cursive};
use cursive::views::*;
use cursive::traits::{Scrollable,Boxable,Identifiable};
/*struct MidiRouter{
    record_receiver:Receiver<Vec<u8>>,
    record_sender:Sender<Vec<u8>>,
    out_conn:MidiOutputConnection,
    in_conn:MidiInputConnection,
}*/

fn main() {
    setup();    
}

fn start(out_port:usize,in_port:usize,path:&str){    
    let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    let midi_out = MidiOutput::new("Output For MidiRouter").unwrap();
    let midi_in = MidiInput::new("Input For MidiRouter").unwrap();

    let mut out = midi_out.connect(out_port, "Out").unwrap();
    let input=midi_in.connect(in_port, "In", move |_stamp, message, record_sender| {
            let mut msg_vec:Vec<u8>=vec![0;message.len()];
             for i in 0..message.len(){
                 msg_vec.insert(i,message[i]);
             }
            if msg_vec.len()!=2{
            tx.send(msg_vec).unwrap();
            }
            match out.send(message){
                Result::Ok(O)=>{},
                Result::Err(E)=>{println!("Error with message {:?}  {:?}",E,message);}
            }
        },())
        .unwrap();    
    record(rx,path);   
}

fn record(rx:Receiver<Vec<u8>>,path:&str){
    let mut builder=SMFBuilder::new();
    builder.add_track();
    let division=120; //120 ticks per beat
    let microsecs_per_tick = 5_000_00 / division as u64;
    let mut events_number=0;
    //let mut start_time = duration_to_micros(&SystemTime::now().elapsed().unwrap());
    loop{ 
        let start_duration = SystemTime::now();      
        let msg=rx.recv().unwrap();

        let delta_time =duration_to_micros(&start_duration.elapsed().unwrap());        
        let delta_ticks=(delta_time)/microsecs_per_tick;
        
        builder.add_event(0,TrackEvent{
            vtime:delta_ticks,
            event:Event::Midi(MidiMessage{data:msg})
        });
        events_number=events_number+1;

        if events_number>=1000{
            break;
        }        
    }
    let mut smf=builder.result();
    smf.division=division;
    let writer=SMFWriter::from_smf(smf);    
    writer.write_to_file(&Path::new(path));

}

fn duration_to_micros(duration:&Duration)->u64{
    (duration.as_secs() * 1_000_000) +
                    duration.subsec_micros() as u64
}


fn setup(){
    let mut siv=Cursive::default();

    let midi_out = MidiOutput::new("Output For MidiRouter").unwrap();
    let midi_in = MidiInput::new("Input For MidiRouter").unwrap();

    let mut out_box=LinearLayout::vertical();
    let mut out_group:RadioGroup<usize>=RadioGroup::new();
    for x in 0..midi_out.port_count() {
        let button=out_group.button(x, midi_out.port_name(x).unwrap());
        out_box.add_child(button);
    }     

    let mut in_box=LinearLayout::vertical();
    let mut in_group:RadioGroup<usize>=RadioGroup::new();
    for x in 0..midi_in.port_count() {
        let button=in_group.button(x, midi_in.port_name(x).unwrap());
        in_box.add_child(button);
    }     
    let mut hbox=LinearLayout::horizontal();
    hbox.add_child(Dialog::around(in_box.scrollable().fixed_size((30,10))).title("MIDI Input"));
    hbox.add_child(Dialog::around(out_box.scrollable().fixed_size((30,10))).title("MIDI Output"));
   
   let file_text=EditView::new().content("./test.mid").with_id("path").fixed_width(40);
   let filebox=LinearLayout::horizontal().child(file_text);

    let mut vbox=LinearLayout::vertical().child(hbox);
    vbox.add_child(Dialog::around(filebox).title("Select A File To Record To"));
    vbox.add_child(Button::new("Start", move |s|{
        let out_port=out_group.selected_id();
        let in_port=in_group.selected_id();
        let pathref = s.call_on_id("path", |view: &mut EditView| {                   
                    view.get_content().clone()
            }).unwrap();
        println!("{}",pathref.as_str());
        //let path=String::from(file_text.get_content());
        spawn(move ||{start(out_port,in_port,"./test.mid")});
    }));
    siv.add_layer(Dialog::around(vbox));
    siv.add_global_callback('q', |s| s.quit());

    siv.run();
    
}