use super::{
    imaging,
    types::{AppResult, GenerateRequest},
};

const CHIIKAWA: &[u8] = include_bytes!("../../assets/characters/chiikawa.png");
const HACHIWARE: &[u8] = include_bytes!("../../assets/characters/hachiware.png");
const USAGI: &[u8] = include_bytes!("../../assets/characters/usagi.png");
const TRIO: &[u8] = include_bytes!("../../assets/characters/trio-fullbody.png");
const MOMONGA: &[u8] = include_bytes!("../../assets/characters/momonga.webp");
const RAKKO: &[u8] = include_bytes!("../../assets/characters/rakko.webp");
const KURIMANJU: &[u8] = include_bytes!("../../assets/characters/kurimanju.webp");
const SHISA: &[u8] = include_bytes!("../../assets/characters/shisa.webp");
const FURUHONYA: &[u8] = include_bytes!("../../assets/characters/furuhonya.webp");

const STYLE: &str = "Draw fully in the visual style, linework and character forms of Chiikawa (ちいかわ), matching the attached artwork closely: squat rounded silhouettes, a very large round head merging into a tiny bean body, extremely short stubby paws and feet, simple black oval eyes with tiny white highlights, a tiny mouth, pink cheeks with short blush hatching. Use dark slightly wobbly hand-drawn outlines, smooth flat white or soft pastel fills, almost no shading and sparse gentle scenery. Match the reference's anatomy, eye spacing, ears and outline weight. Clean digital 2D drawing: no grain, mottling, paper texture, watercolor, halftone, photorealism, 3D, glossy toys, long limbs, elaborate anime eyes or gradients. Never copy labels, captions, names, pronunciation guides, signatures or watermarks from the references.";

pub fn reference_bytes(character: &str) -> Option<&'static [u8]> {
    Some(match character {
        "own" | "trio" => TRIO,
        "chiikawa" => CHIIKAWA,
        "hachiware" => HACHIWARE,
        "usagi" => USAGI,
        "momonga" => MOMONGA,
        "rakko" => RAKKO,
        "kurimanju" => KURIMANJU,
        "shisa" => SHISA,
        "furuhonya" => FURUHONYA,
        _ => return None,
    })
}

fn style_images(request: &GenerateRequest) -> Vec<&'static [u8]> {
    let photos = if request.mode == "photo" {
        request.photos.len()
    } else {
        0
    };
    if ["own", "trio"].contains(&request.character.as_str())
        && photos + 3 + usize::from(request.custom_reference.is_some()) <= 10
    {
        vec![CHIIKAWA, HACHIWARE, USAGI]
    } else {
        vec![reference_bytes(&request.character).unwrap_or(TRIO)]
    }
}

pub fn quote_input(request: &GenerateRequest) -> AppResult<(String, usize)> {
    request.validate()?;
    let styles = style_images(request).len();
    let photos = if request.mode == "photo" {
        request.photos.len()
    } else {
        0
    };
    Ok((
        build_prompt(request, styles)?,
        photos + styles + usize::from(request.custom_reference.is_some()),
    ))
}

pub fn prepare(request: &GenerateRequest) -> AppResult<(String, Vec<String>)> {
    request.validate()?;
    let styles = style_images(request);
    let prompt = build_prompt(request, styles.len())?;
    let mut refs = Vec::new();
    if request.mode == "photo" {
        for path in &request.photos {
            refs.push(imaging::prepare_path(path)?);
        }
    }
    for bytes in styles {
        refs.push(imaging::prepare_bytes(bytes)?);
    }
    if let Some(path) = &request.custom_reference {
        refs.push(imaging::prepare_path(path)?);
    }
    if refs.len() > 10 {
        return Err("too_many_photos".into());
    }
    Ok((prompt, refs))
}

pub fn build_prompt(request: &GenerateRequest, style_count: usize) -> AppResult<String> {
    request.validate()?;
    let photo = request.mode == "photo";
    let mut parts = vec![STYLE.to_string()];
    if photo {
        let count = request.photos.len();
        parts.push(format!("Images 1 through {count} are SOURCE PHOTOGRAPHS. Combine subjects from all of them into ONE coherent scene, never a collage or split panels. Merge repeated views of the same subject unless asked otherwise. Preserve the source subject count and recognizable clothing, colors and accessories."));
        if request.full_creature {
            parts.push("Fully replace each person's anatomy with a complete NON-HUMAN Chiikawa creature: one wide rounded head/bean-body silhouette about 1.3-1.6 head-heights tall, animal ears, tiny side paws, tiny rounded feet directly under the body, white or pastel fur. No human neck, shoulders, waist, long limbs, fingers, skin, nose, lips, jawline or hair cap. Never draw chibi humans or people in mascot suits. Keep identity ONLY through miniature clothes, main colors, glasses, hats, jewelry and accessories fitted to the creature. Replace hair with ears/fur but retain hair accessories. Adapt positions and gestures to stubby paws. Show enough body to read as a complete creature. Anatomy stays fully non-human at every strength.".into());
            parts.push(
                if request.strength >= 70.0 {
                    "Simplify clothing to key color blocks and accessories."
                } else {
                    "Retain more recognizable clothing markings and accessory details."
                }
                .into(),
            );
        } else {
            parts.push("Completely redraw the photograph with Chiikawa shapes while preserving arrangement, subject count, pose intention and identifying details.".into());
            parts.push(if request.strength >= 70.0 {
                "Fully replace anatomy with squat Chiikawa creature anatomy; reduce hair/clothes to a few identifying shapes."
            } else if request.strength >= 35.0 {
                "Turn every subject into a rounded Chiikawa creature while retaining simplified hairstyle and clothing for recognition."
            } else {
                "Retain more recognizable hairstyle, outfit and pose, with short rounded Chiikawa proportions and very simple faces."
            }.into());
        }
    } else {
        parts.push("Create a NEW scene from the description. References guide character designs and drawing style, not the composition.".into());
    }
    let first = if photo { request.photos.len() + 1 } else { 1 };
    parts.push(format!("Images {first} through {} are character/style references. Do not add these reference characters as extra subjects in a photo transformation.", first + style_count - 1));
    if request.custom_reference.is_some() {
        parts.push("The last image is a supplementary visual reference. Use its requested colors/accessories without overriding Chiikawa anatomy, the source subject count or the clean flat drawing style.".into());
    }
    parts.push(match request.character.as_str() {
        "chiikawa" => "Chiikawa: white round creature with small round ears, simple gentle face, pink cheeks and tiny paws.",
        "hachiware" => "Hachiware: white creature, blue split forehead marking, blue triangular ears, tiny paws and an optimistic face.",
        "usagi" => "Usagi: pale cream-yellow rabbit-like creature with long upright round-tipped ears and a lively face.",
        "momonga" => "Momonga: white flying-squirrel creature, very large fluffy pale-blue tail behind the head, big round ears, forehead tuft, pale-blue muzzle patch, glossy simple oval eyes and pink cheeks. Follow the supplied design.",
        "rakko" => "Rakko: cream-beige sea otter creature with a star-shaped scar above one eye, white scarf and cape, tiny dark paws and a sword at the side. Follow the supplied design.",
        "kurimanju" => "Kurimanju: cream-beige round creature with a large chestnut-brown cap patch on the crown, small rounded ears, round black eyes, tiny muzzle, pink nose area and blushing cheeks. Follow the supplied design.",
        "shisa" => "Shisa: pale cream guardian-lion creature, small pointed ears, orange curled mane framing the sides, curled orange eyebrows, short paws and pink cheeks. Follow the supplied design.",
        "furuhonya" => "Furuhonya (Kani): pale pink creature with two rounded upright pink ears, pink forehead marking, tiny eyebrows, cheerful closed crescent eyes, tiny mouth and pink cheeks. Follow the supplied design.",
        "trio" if photo => "Use Chiikawa, Hachiware and Usagi as design alternatives for the source subjects; preserve the source subject count even if different from three.",
        "trio" => "Include the three friends Chiikawa, Hachiware and Usagi with their distinct reference designs.",
        _ => "Create an original small creature in the Chiikawa world, preserving identifying colors and accessories.",
    }.into());
    parts.push(
        match request.background.as_str() {
            "white" => "Plain white background with generous empty space.",
            "meadow" => "Sparse mint meadow, tiny flowers, pale blue sky.",
            "room" => "Simple cozy pastel room, small rounded furniture and sparse details.",
            "sticker" => {
                "One centered sticker on solid white, with a clean silhouette and no text."
            }
            _ => {
                "Redraw the requested setting using sparse flat pastel shapes and simple linework."
            }
        }
        .into(),
    );
    parts.push(
        match request.emotion.as_str() {
            "happy" => "Happy eyes and tiny smiling mouth.",
            "shy" => "Shy gentle face and pink cheeks.",
            "tears" => "Small anxious tears and a tender expression.",
            "surprised" => "Wide simple eyes and tiny surprised mouth.",
            _ => "Use an expression appropriate to the scene.",
        }
        .into(),
    );
    parts.push(format!(
        "Scene / additional instructions: {}",
        if request.description.trim().is_empty() {
            "A gentle everyday moment."
        } else {
            request.description.trim()
        }
    ));
    let prompt = parts.join("\n");
    if prompt.chars().count() > 4096 {
        return Err("description_too_long".into());
    }
    Ok(prompt)
}

#[cfg(test)]
pub fn reference_smoke_test() -> AppResult<()> {
    for id in super::CHARACTER_IDS {
        imaging::decode(reference_bytes(id).ok_or("invalid_settings")?)?;
    }
    Ok(())
}
