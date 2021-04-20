extern crate diacritics;
extern crate metaflac;

use metaflac::Tag;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if let Some(file) = args.get(1) {
        let mut tag = Tag::read_from_path(&file).unwrap();

        for comment in &tag.vorbis_comments_mut().comments {
            println!(
                "{}: {}",
                comment.0,
                diacritics::remove_diacritics(&comment.1[0])
            );
        }

        tag.save().unwrap();
    } else {
        println!("No file specified.")
    }
}
