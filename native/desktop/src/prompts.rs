use crate::{
    imaging,
    types::{AppResult, GenerateRequest},
};

const CHIIKAWA: &[u8] = include_bytes!("../../shared/assets/chiikawa.png");
const HACHIWARE: &[u8] = include_bytes!("../../shared/assets/hachiware.png");
const USAGI: &[u8] = include_bytes!("../../shared/assets/usagi.png");
const TRIO: &[u8] = include_bytes!("../../shared/assets/trio-fullbody.png");

const STYLE: &str = "Draw in the visual style and character shapes of Chiikawa (ちいかわ). Use the attached official reference artwork as the visual guide: rounded squat silhouettes, very large rounded head merging into a small bean-shaped body, extremely short stubby arms and feet, simple black oval eyes with small white highlights, tiny mouth, pink cheek patches with short blush hatching. Dark slightly wobbly hand-drawn outlines, flat white or soft pastel fills, almost no shading, sparse gentle scenery, 2D manga/anime drawing. Match the reference proportions, face placement and linework. Avoid realistic human anatomy, long limbs, detailed noses, realistic fur, 3D rendering, glossy toy surfaces, elaborate anime eyes, gradients, text, signatures and watermarks. References guide style only; do not copy their labels or typography.";

const CODEX_STYLE: &str = "Faithfully reproduce the drawing language of the official Chiikawa artwork: exact head/body silhouettes, eye spacing, tiny mouth placement, ear shapes, paw lengths, dark outline weight, limited pastel palette and blush hatching. Treat the official images as the style specification, not loose inspiration. Render a clean digital 2D drawing: every bounded region has a smooth uniform solid color. White fur stays clean white; pastel fields have no mottling. Crisp anti-aliased outlines and minimal shading. No film grain, paper/canvas texture, watercolor, pencil shading, stippling, halftone, dithering, dust, JPEG artifacts, noise or added texture. Intentional cheek hatch strokes and facial ink are drawing details, not noise. Never substitute generic kawaii, chibi-anime people, plush toys, 3D or painterly art. Take subject count, pose, clothes and accessories from source photos, but drawing style from official artwork only. Never copy photographic grain or lighting.";

pub fn style_reference_count(character: &str, photo_count: usize, custom: bool) -> usize {
    if ["own", "trio"].contains(&character) && photo_count + 3 + usize::from(custom) <= 10 {
        3
    } else {
        1
    }
}

pub fn prepare(request: &GenerateRequest) -> AppResult<(String, Vec<String>)> {
    request.validate()?;
    let photo_count = if request.mode == "photo" {
        request.photos.len()
    } else {
        0
    };
    let mut refs = Vec::new();
    if request.mode == "photo" {
        for photo in &request.photos {
            refs.push(imaging::prepare(photo)?);
        }
    }
    let custom = request.custom_reference.as_ref().filter(|s| !s.is_empty());
    let count = style_reference_count(&request.character, photo_count, custom.is_some());
    let style = if ["own", "trio"].contains(&request.character.as_str()) {
        if count == 3 {
            vec![CHIIKAWA, HACHIWARE, USAGI]
        } else {
            vec![TRIO]
        }
    } else {
        vec![match request.character.as_str() {
            "hachiware" => HACHIWARE,
            "usagi" => USAGI,
            _ => CHIIKAWA,
        }]
    };
    for bytes in style {
        refs.push(imaging::prepare_bytes(bytes)?);
    }
    if let Some(custom) = custom {
        refs.push(imaging::prepare(custom)?);
    }
    Ok((build_prompt(request, count)?, refs))
}

pub fn build_prompt(r: &GenerateRequest, style_count: usize) -> AppResult<String> {
    r.validate()?;
    let is_photo = r.mode == "photo";
    let mut parts: Vec<String> = vec![STYLE.into()];
    if r.provider == "codex" {
        parts.insert(0, CODEX_STYLE.into());
    }
    if is_photo {
        parts.push(source_roles(r.photos.len()));
        if r.full_creature {
            parts.push("Fully replace every person's anatomy with a complete non-human Chiikawa creature: one wide rounded head-and-bean-body silhouette, about 1.3-1.6 head heights tall excluding ears. Animal ears on top, tiny side paws, tiny rounded feet directly under the body. No human neck, shoulders, waist, long limbs, fingers, hairstyle or hair cap. White or pale pastel fur, simple black eyes, tiny animal muzzle/mouth and pink cheek ovals with hatch marks. No human skin, nose, lips, jawline, chibi humans, dolls or people in mascot suits. Keep exactly one transformed creature per distinct source subject.".into());
            parts.push("Preserve identity ONLY through miniature clothing, main color blocks, glasses, hats, jewelry and distinctive accessories fitted to the creature. Replace hair with ears/fur, retaining hair accessories themselves. Keep positions, gesture intention and held objects, adapting poses to stubby paws. Show enough body to read as a complete creature. Creature anatomy stays fully non-human at every strength setting.".into());
            parts.push(format!(
                "Outfit detail: {}.",
                if r.strength >= 70.0 {
                    "few main color blocks and key accessories"
                } else if r.strength >= 35.0 {
                    "recognizable simplified garment details"
                } else {
                    "more garment markings and accessory details"
                }
            ));
        } else {
            parts.push("Completely redraw the photograph in Chiikawa shapes. Preserve the number of subjects, pose, arrangement, recognizable accessories and main colors.".into());
            parts.push(if r.strength >= 70.0 { "Fully replace human/animal anatomy with squat Chiikawa creature anatomy. Reduce hair and clothing to a few simple identifying shapes." } else if r.strength >= 35.0 { "Turn every subject into a rounded Chiikawa creature while retaining simplified hairstyle and clothing for recognition." } else { "Keep more recognizable hairstyle, outfit and pose, but still use short rounded Chiikawa proportions and simple faces." }.into());
        }
    } else {
        parts.push("Create a NEW scene from the description. Reference images are character and drawing guides only; do not reproduce their layout.".into());
    }
    let first = if is_photo { r.photos.len() + 1 } else { 1 };
    parts.push(format!("Images {first} through {} are official full-body Chiikawa style and shape references. Never add the style-reference characters as extra subjects.", first + style_count - 1));
    if r.custom_reference.as_ref().is_some_and(|s| !s.is_empty()) {
        parts.push(if r.provider == "codex" { "The last image is supplementary: use its requested colors/accessories, never its texture or a conflicting drawing style." } else if is_photo && r.full_creature { "The last image guides palette and linework, never overriding the required creature anatomy or subject count." } else { "The last image is the user's preferred visual reference; give its drawing style priority." }.into());
    }
    parts.push(match r.character.as_str() {
        "chiikawa" => "Chiikawa: white round creature, small rounded ears, shy gentle expression, pink cheeks, tiny paws.",
        "hachiware" => "Hachiware: white creature with blue split forehead marking and blue triangular ears, optimistic expression, tiny paws.",
        "usagi" => "Usagi: pale cream-yellow creature with long upright rounded bunny-like ears, lively playful expression, tiny paws.",
        "trio" if is_photo && r.full_creature => "Use the three friends as alternative creature designs for the source subjects, preserving the source subject count even when it differs from three.",
        "trio" => "Include the three friends Chiikawa, Hachiware and Usagi, faithfully following their distinct reference designs.",
        _ => "Create an original little creature in the Chiikawa world; retain the subject's identifying colors and accessories.",
    }.into());
    parts.push(match r.background.as_str() {
        "white" => "Plain white background with generous empty space.",
        "meadow" => "A sparse soft mint meadow, a few tiny flowers and a pale blue sky.",
        "room" => "A simple cozy pastel room, small rounded furniture, minimal decorative details.",
        "sticker" => "One centered sticker composition on a solid white background, clean silhouette, no text. White background, not transparency.",
        _ => "Keep the requested setting, redraw it with sparse flat pastel shapes and simple linework.",
    }.into());
    parts.push(
        match r.emotion.as_str() {
            "happy" => "Happy sparkling eyes and a tiny smiling mouth.",
            "shy" => "Shy gentle expression, pink blushing cheeks.",
            "tears" => "Small anxious tears, trembling tiny mouth, tender expression.",
            "surprised" => "Wide simple eyes and a tiny surprised mouth.",
            _ => "Use an expression appropriate to the scene.",
        }
        .into(),
    );
    parts.push(format!(
        "Scene / additional instructions: {}",
        if r.description.trim().is_empty() {
            "A gentle everyday moment."
        } else {
            r.description.trim()
        }
    ));
    let prompt = parts.join("\n");
    if r.provider == "elyum" && prompt.chars().count() > 4096 {
        return Err("description_too_long".into());
    }
    Ok(prompt)
}

fn source_roles(count: usize) -> String {
    if count == 1 {
        return "Image 1 is the source photograph. All subsequent images are art-style references."
            .into();
    }
    format!("Images 1 through {count} are source photographs; all subsequent images are art-style references. Combine subjects from ALL source photographs into ONE coherent new scene, not a collage or split panels. Use the description for their arrangement. If photos show the same subject, merge those views into one character unless asked otherwise. Never add style-reference characters as extra subjects.")
}

pub fn reference_smoke_test() -> AppResult<()> {
    for bytes in [CHIIKAWA, HACHIWARE, USAGI, TRIO] {
        imaging::decode(bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> GenerateRequest {
        serde_json::from_value(serde_json::json!({"provider":"elyum","mode":"photo","description":"picnic","photos":["photo1","photo2"],"customReference":null,"character":"own","background":"scene","emotion":"scene","strength":50,"fullCreature":false,"aspectRatio":"1:1","resolution":"2K"})).unwrap()
    }
    #[test]
    fn classic_default_and_full_creature_are_distinct() {
        let mut r = request();
        let classic = build_prompt(&r, 3).unwrap();
        assert!(classic.contains("retaining simplified hairstyle"));
        r.full_creature = true;
        let full = build_prompt(&r, 3).unwrap();
        assert!(full.contains("No human neck"));
        assert!(!full.contains("retaining simplified hairstyle"));
        assert!(full.contains("ONE coherent new scene"));
    }
    #[test]
    fn eight_photos_and_custom_fit_provider_limit() {
        assert_eq!(style_reference_count("own", 8, true), 1);
        assert_eq!(style_reference_count("trio", 6, true), 3);
    }
    #[test]
    fn codex_includes_clean_drawing_direction() {
        let mut r = request();
        r.provider = "codex".into();
        assert!(build_prompt(&r, 3).unwrap().contains("No film grain"));
    }
    #[test]
    fn references_are_real_decodable_images() {
        reference_smoke_test().unwrap();
    }
}
