#[cfg(feature = "amethyst")]
extern crate serde;

#[cfg(feature = "amethyst")]
#[macro_use]
extern crate serde_derive;

extern crate smallvec;
extern crate twox_hash;

mod format;
mod pack;
mod sprite;

pub use {
    format::Format,
    pack::{
        maxrects::{MaxrectsOptions, MaxrectsPacker},
        simple::SimplePacker,
        Packer, PackerResult,
    },
    sprite::{InputSprite, Sprite, SpriteAnchor, SpriteData},
};

#[cfg(feature = "amethyst")]
pub use format::amethyst::{AmethystFormat, SerializedSpriteSheet, SpritePosition};
pub use format::named::AmethystNamedFormat;

use sprite::{create_pixel_buffer, write_sprite};

use smallvec::SmallVec;
use std::collections::hash_map::HashMap;
use std::hash::BuildHasherDefault;
use twox_hash::XxHash64;

#[derive(Debug, Clone)]
pub struct SpriteSheet {
    pub bytes: Vec<u8>,
    pub stride: usize,
    pub dimensions: (u32, u32),
    anchors: Vec<SpriteAnchor>,
}

pub fn pack<P: Packer>(
    input: Vec<InputSprite>,
    stride: usize,
    options: P::Options,
) -> Vec<SpriteSheet> {
    let mut hashes: HashMap<&[u8], usize, BuildHasherDefault<XxHash64>> = Default::default();
    let mut aliases: Vec<SmallVec<[usize; 1]>> =
        (0..input.len()).map(|_| SmallVec::new()).collect();

    for (id, sprite) in input.iter().enumerate() {
        let alias_id = hashes.entry(sprite.bytes.as_slice()).or_insert(id);
        aliases[*alias_id].push(id);
    }

    let sprites = input
        .into_iter()
        .enumerate()
        .filter(|(id, _)| !aliases[*id].is_empty())
        .map(|(id, sprite)| Sprite::from_input(id, sprite))
        .collect::<Vec<Sprite>>();
        
    let sprite_data = sprites
        .iter()
        .map(|it| it.data)
        .collect::<Vec<SpriteData>>();

    let packer_result = P::pack(&sprite_data, options);

    packer_result
        .into_iter()
        .map(|mut sheet| {
            let mut buffer = create_pixel_buffer(sheet.dimensions, stride);
            let mut additional_anchors = Vec::<SpriteAnchor>::new();
            for anchor in &sheet.anchors {
                write_sprite(
                    &mut buffer,
                    sheet.dimensions,
                    stride,
                    &sprites[anchor.id],
                    &anchor,
                );
                additional_anchors.extend(
                    aliases
                        .iter()
                        .flat_map(|aliases| aliases.iter().skip(1))
                        .map(|alias| SpriteAnchor {
                            id: *alias,
                            ..*anchor
                        }),
                );
            }
            sheet.anchors.extend(additional_anchors);

            SpriteSheet {
                bytes: buffer,
                stride: stride,
                dimensions: sheet.dimensions,
                anchors: sheet.anchors,
            }
        })
        .collect()
}

pub fn encode<F>(sprite_sheet: &SpriteSheet, options: F::Options) -> F::Data
where
    F: Format,
{
    F::encode(sprite_sheet.dimensions, &sprite_sheet.anchors, options)
}
