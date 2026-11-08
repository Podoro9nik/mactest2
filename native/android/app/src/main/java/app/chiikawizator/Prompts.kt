package app.chiikawizator

object Prompts {
    private const val STYLE = "Draw in the visual style and character shapes of Chiikawa (ちいかわ). Use the attached official reference artwork as the visual guide: rounded squat silhouettes, very large rounded head merging into a small bean-shaped body, extremely short stubby arms and feet, simple black oval eyes with small white highlights, tiny mouth, pink cheek patches with short blush hatching. Dark slightly wobbly hand-drawn outlines, flat white or soft pastel fills, almost no shading, sparse gentle scenery, 2D manga/anime drawing. Match the reference proportions, face placement and linework. Smooth solid fills, clean white fur, crisp anti-aliased outlines. No grain, paper texture, noise, watercolor, 3D, gradients, text, signatures or watermarks. The reference images guide style only; do not copy their labels or typography."
    private val characters = mapOf(
        "own" to "Create an original little creature in the Chiikawa world; retain the subject's identifying colors and accessories.",
        "chiikawa" to "Chiikawa: white round creature, small rounded ears, shy gentle expression, pink cheeks, tiny paws.",
        "hachiware" to "Hachiware: white creature with blue split forehead marking and blue triangular ears, optimistic expression, tiny paws.",
        "usagi" to "Usagi: pale cream-yellow creature with long upright rounded bunny-like ears, lively playful expression, tiny paws.",
        "trio" to "Include the three friends Chiikawa, Hachiware and Usagi, faithfully following their distinct reference designs.")
    private val backgrounds = mapOf("scene" to "Keep the requested setting, redraw it with sparse flat pastel shapes and simple linework.",
        "white" to "Plain white background with generous empty space.", "meadow" to "A sparse soft mint meadow, a few tiny flowers and a pale blue sky.",
        "room" to "A simple cozy pastel room, small rounded furniture, minimal decorative details.",
        "sticker" to "One centered sticker composition on a solid white background, clean silhouette, no text. White background, not transparency.")
    private val emotions = mapOf("scene" to "Use an expression appropriate to the scene.", "happy" to "Happy sparkling eyes and a tiny smiling mouth.",
        "shy" to "Shy gentle expression, pink blushing cheeks.", "tears" to "Small anxious tears, trembling tiny mouth, tender expression.", "surprised" to "Wide simple eyes and a tiny surprised mouth.")

    fun references(character: String, photoCount: Int, custom: Boolean): List<String> {
        val selected = when (character) {
            "chiikawa", "hachiware", "usagi" -> listOf("$character.png")
            else -> listOf("chiikawa.png", "hachiware.png", "usagi.png")
        }
        return if (photoCount + selected.size + (if (custom) 1 else 0) > 10) listOf("trio-fullbody.png") else selected
    }

    fun build(r: GenerationRequest): String {
        if (r.strength !in 0..100 || r.character !in characters || r.background !in backgrounds || r.emotion !in emotions) throw StudioException("invalid_request")
        if (r.aspectRatio !in setOf("1:1", "3:4", "4:3", "9:16", "16:9") || r.resolution !in setOf("1K", "2K", "4K")) throw StudioException("invalid_request")
        if (r.photo && r.photos.size !in 1..8) throw StudioException("photos_required")
        if (!r.photo && r.description.trim().length < 3) throw StudioException("description_required")
        val parts = mutableListOf(STYLE)
        if (r.photo) {
            parts += if (r.photos.size == 1) "Image 1 is the source photograph. All subsequent images are art-style references." else
                "Images 1 through ${r.photos.size} are source photographs; all subsequent images are art-style references. Combine subjects from ALL source photographs into ONE coherent new scene, not a collage or split panels. Use the scene description for their arrangement. If multiple photos show the same subject, merge those views into one character unless the description asks otherwise. Never add the style-reference characters as extra subjects."
            if (r.fullCreature) {
                parts += "Keep exactly one transformed creature per distinct source subject. Fully replace every person's anatomy with a complete non-human Chiikawa creature: one wide rounded head-and-bean-body silhouette, about 1.3-1.6 head heights tall excluding ears. Animal ears on top, tiny side paws, tiny rounded feet directly under the body. No human neck, shoulders, waist, long limbs, fingers, hairstyle or hair cap. White or pale pastel fur, simple black eyes, tiny animal muzzle/mouth and pink cheek ovals with hatch marks. No human skin, nose, lips, jawline, chibi humans, dolls or people in mascot suits."
                parts += "Preserve identity ONLY through miniature clothing, main color blocks, glasses, hats, jewelry and distinctive accessories fitted to the creature. Replace hair with ears/fur; retain hair accessories themselves. Keep subject positions, gesture intention and held objects, adapting poses to stubby paws. Show enough body to read as a complete creature. Creature anatomy stays fully non-human at every setting."
            } else {
                parts += "Completely redraw the photograph in Chiikawa shapes. Preserve the number of subjects, their pose, arrangement, recognizable accessories and main colors."
                parts += when {
                    r.strength >= 70 -> "Fully replace human/animal anatomy with squat Chiikawa creature anatomy. Reduce hair and clothing to a few simple identifying shapes."
                    r.strength >= 35 -> "Turn every subject into a rounded Chiikawa creature while retaining simplified hairstyle and clothing for recognition."
                    else -> "Keep more recognizable hairstyle, outfit and pose, but still use short rounded Chiikawa proportions and simple faces."
                }
            }
        } else parts += "Create a NEW scene from the description. Reference images are only character and drawing guides; do not reproduce the reference layout."
        if (r.photo && r.fullCreature && r.character == "trio") parts += "Use the three friends as alternative creature designs for the source subjects, preserving the source subject count even when it differs from three."
        else parts += characters.getValue(r.character)
        if (r.customReference != null) parts += "The last image is a supplementary visual reference for palette, outfit and linework. Keep the required Chiikawa anatomy and subject count."
        parts += backgrounds.getValue(r.background)
        parts += emotions.getValue(r.emotion)
        parts += "Scene / additional instructions: " + r.description.trim().ifEmpty { "A gentle everyday moment." }
        return parts.joinToString("\n").also { if (it.length > 4096) throw StudioException("prompt_too_long") }
    }
}
