#[derive(Clone, Copy, Default)]
struct RoleIconSet {
    skill: Option<TextureRegion>,
    ultimate: Option<TextureRegion>,
}

#[derive(Clone, Copy, Default)]
struct RecluseIconSet {
    atlas: Option<TextureId>,
}

#[derive(Clone, Copy)]
struct TextureRegion {
    texture_id: TextureId,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
}

impl TextureRegion {
    fn atlas_slot(texture_id: TextureId, slot: usize, total_slots: usize) -> Self {
        let slot_w = 1.0 / total_slots as f32;
        let min_x = slot as f32 * slot_w;
        Self {
            texture_id,
            uv_min: [min_x, 0.0],
            uv_max: [min_x + slot_w, 1.0],
        }
    }
}

impl RecluseIconSet {
    fn magic_region(&self, index: usize) -> Option<TextureRegion> {
        self.atlas.map(|texture_id| {
            TextureRegion::atlas_slot(
                texture_id,
                RECLUSE_MAGIC_ATLAS_FIRST_SLOT + index,
                RECLUSE_ATLAS_SLOTS,
            )
        })
    }
}

#[derive(Clone, Copy, Default)]
struct EffectIconSet {
    atlas: Option<TextureId>,
}

impl EffectIconSet {
    fn ultimate_on(&self) -> [Option<TextureRegion>; 3] {
        [
            self.region(EFFECT_ULTIMATE_ON1_ATLAS_SLOT),
            self.region(EFFECT_ULTIMATE_ON2_ATLAS_SLOT),
            None,
        ]
    }

    fn ultimate_on_region(&self, index: usize) -> Option<TextureRegion> {
        match index {
            0 => self.region(EFFECT_ULTIMATE_ON1_ATLAS_SLOT),
            1 => self.region(EFFECT_ULTIMATE_ON2_ATLAS_SLOT),
            _ => None,
        }
    }

    fn undertaker_skill(&self) -> [Option<TextureRegion>; 3] {
        [
            self.region(EFFECT_UNDERTAKER_SKILL_SP1_ATLAS_SLOT),
            self.region(EFFECT_UNDERTAKER_SKILL_SP2_ATLAS_SLOT),
            None,
        ]
    }

    fn undertaker_ultimate(&self) -> [Option<TextureRegion>; 3] {
        [
            self.region(EFFECT_UNDERTAKER_ULTIMATE_SP1_ATLAS_SLOT),
            self.region(EFFECT_UNDERTAKER_ULTIMATE_SP2_ATLAS_SLOT),
            self.region(EFFECT_UNDERTAKER_ULTIMATE_SP3_ATLAS_SLOT),
        ]
    }

    fn region(&self, slot: usize) -> Option<TextureRegion> {
        self.atlas
            .map(|texture_id| TextureRegion::atlas_slot(texture_id, slot, EFFECT_ATLAS_SLOTS))
    }
}

#[derive(Clone, Copy, Default)]
struct ScholarIconSet {
    lens: Option<TextureId>,
    enemy_atlas: Option<TextureId>,
}

#[derive(Clone, Copy, Default)]
struct IroneyeIconSet {
    atlas: Option<TextureId>,
}

#[derive(Clone, Copy, Default)]
struct RevenantIconSet {
    summon_atlas: Option<TextureId>,
}

struct IconSet {
    roles: [RoleIconSet; ROLE_COUNT],
    effects: EffectIconSet,
    ironeye: IroneyeIconSet,
    revenant: RevenantIconSet,
    recluse: RecluseIconSet,
    scholar: ScholarIconSet,
    wylder_skill_lock: Option<TextureId>,
    multi_locking: Option<TextureId>,
}

impl Default for IconSet {
    fn default() -> Self {
        Self {
            roles: [RoleIconSet::default(); ROLE_COUNT],
            effects: EffectIconSet::default(),
            ironeye: IroneyeIconSet::default(),
            revenant: RevenantIconSet::default(),
            recluse: RecluseIconSet::default(),
            scholar: ScholarIconSet::default(),
            wylder_skill_lock: None,
            multi_locking: None,
        }
    }
}

impl IconSet {
    fn load(&mut self, render_context: &mut dyn RenderContext) {
        let skill_atlas = load_role_icon_atlas(render_context, RoleIconAtlasKind::Skill);
        let ultimate_atlas = load_role_icon_atlas(render_context, RoleIconAtlasKind::Ultimate);
        for config in ROLE_CONFIGS {
            let index = config.role as usize;
            self.roles[index].skill = skill_atlas
                .map(|texture_id| TextureRegion::atlas_slot(texture_id, index, ROLE_COUNT));
            self.roles[index].ultimate = ultimate_atlas
                .map(|texture_id| TextureRegion::atlas_slot(texture_id, index, ROLE_COUNT));
        }

        self.effects = EffectIconSet {
            atlas: load_effect_atlas(render_context),
        };
        self.ironeye = IroneyeIconSet {
            atlas: load_ironeye_atlas(render_context),
        };
        self.revenant = RevenantIconSet {
            summon_atlas: load_revenant_summon_atlas(render_context),
        };
        self.recluse = RecluseIconSet {
            atlas: load_recluse_atlas(render_context),
        };
        self.scholar = ScholarIconSet {
            lens: load_raw_texture(
                render_context,
                include_bytes!("../assets/Scholar_Skill_Lock.png"),
            ),
            enemy_atlas: load_scholar_enemy_atlas(render_context),
        };
        self.wylder_skill_lock = load_raw_texture(
            render_context,
            include_bytes!("../assets/Wylder_Skill_Lock.png"),
        );
        self.multi_locking = load_raw_texture(
            render_context,
            include_bytes!("../assets/MultipleLocking.png"),
        );
    }

    fn role(&self, role: Role) -> RoleIconSet {
        self.roles[role as usize]
    }
}

#[derive(Clone, Copy)]
enum RoleIconAtlasKind {
    Skill,
    Ultimate,
}

impl RoleIconAtlasKind {
    fn crop_inset(self) -> u32 {
        match self {
            RoleIconAtlasKind::Skill => 48,
            RoleIconAtlasKind::Ultimate => 28,
        }
    }

    fn bytes(self, config: &RoleConfig) -> &'static [u8] {
        match self {
            RoleIconAtlasKind::Skill => config.skill_bytes,
            RoleIconAtlasKind::Ultimate => config.ultimate_bytes,
        }
    }

    fn label(self) -> &'static str {
        match self {
            RoleIconAtlasKind::Skill => "Role skill atlas",
            RoleIconAtlasKind::Ultimate => "Role ultimate atlas",
        }
    }
}

fn load_role_icon_atlas(
    render_context: &mut dyn RenderContext,
    kind: RoleIconAtlasKind,
) -> Option<TextureId> {
    let crop_inset = kind.crop_inset();
    let slot_side = 256 - crop_inset * 2;
    let mut atlas = image::RgbaImage::new(slot_side * ROLE_COUNT as u32, slot_side);
    for config in ROLE_CONFIGS {
        let image = ImageReader::new(Cursor::new(kind.bytes(&config)))
            .with_guessed_format()
            .unwrap_or_else(|_| panic!("{} source format should be readable", kind.label()))
            .decode()
            .unwrap_or_else(|_| panic!("{} source should decode", kind.label()))
            .into_rgba8();
        let cropped = crop_center_square(&image, crop_inset);
        assert_eq!(
            cropped.width(),
            slot_side,
            "{} slot width should match crop",
            kind.label()
        );
        assert_eq!(
            cropped.height(),
            slot_side,
            "{} slot height should match crop",
            kind.label()
        );
        let mut rgba = cropped.into_raw();
        apply_circle_alpha_mask(&mut rgba, slot_side, slot_side);
        let masked = image::RgbaImage::from_raw(slot_side, slot_side, rgba)
            .unwrap_or_else(|| panic!("{} masked image should be valid", kind.label()));
        image::imageops::overlay(
            &mut atlas,
            &masked,
            (config.role as i64) * slot_side as i64,
            0,
        );
    }
    render_context
        .load_texture(atlas.as_raw(), atlas.width(), atlas.height())
        .ok()
}

fn load_raw_texture(render_context: &mut dyn RenderContext, bytes: &[u8]) -> Option<TextureId> {
    let image = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .expect("HUD effect format should be readable")
        .decode()
        .expect("HUD effect should decode")
        .into_rgba8();
    let width = image.width();
    let height = image.height();
    render_context
        .load_texture(image.as_raw(), width, height)
        .ok()
}

fn load_ironeye_atlas(render_context: &mut dyn RenderContext) -> Option<TextureId> {
    load_horizontal_atlas_256(
        render_context,
        &[
            include_bytes!("../assets/Ironeye_Reticle1.png").as_slice(),
            include_bytes!("../assets/Ironeye_Reticle2.png").as_slice(),
            include_bytes!("../assets/Ironeye_Reticle3.png").as_slice(),
            include_bytes!("../assets/Ironeye_weakness1.png").as_slice(),
            include_bytes!("../assets/Ironeye_weakness2.png").as_slice(),
            include_bytes!("../assets/Ironeye_weakness3.png").as_slice(),
            include_bytes!("../assets/Ironeye_weakness4.png").as_slice(),
        ],
        "Ironeye atlas",
    )
}

fn load_effect_atlas(render_context: &mut dyn RenderContext) -> Option<TextureId> {
    load_horizontal_atlas_256_with_alpha(
        render_context,
        &[
            (include_bytes!("../assets/Ultimate_on1.png").as_slice(), 1.0),
            (include_bytes!("../assets/Ultimate_on2.png").as_slice(), 2.0),
            (
                include_bytes!("../assets/Undertaker_Skill_SP1.png").as_slice(),
                1.0,
            ),
            (
                include_bytes!("../assets/Undertaker_Skill_SP2.png").as_slice(),
                2.0,
            ),
            (
                include_bytes!("../assets/Undertaker_Ultimate_SP1.png").as_slice(),
                1.0,
            ),
            (
                include_bytes!("../assets/Undertaker_Ultimate_SP2.png").as_slice(),
                4.0,
            ),
            (
                include_bytes!("../assets/Undertaker_Ultimate_SP3.png").as_slice(),
                4.0,
            ),
        ],
        "Effect atlas",
    )
}

fn load_scholar_enemy_atlas(render_context: &mut dyn RenderContext) -> Option<TextureId> {
    load_horizontal_atlas_256(
        render_context,
        &[
            include_bytes!("../assets/Scholar_Skill_Lock_Enemy_Full.png").as_slice(),
            include_bytes!("../assets/Scholar_Skill_Lock_Enemy_Pointer.png").as_slice(),
            include_bytes!("../assets/Scholar_Skill_Lock_Enemy_PointerBack.png").as_slice(),
            include_bytes!("../assets/Scholar_Skill_Lock_Enemy_White.png").as_slice(),
            include_bytes!("../assets/Scholar_Skill_Lock_Enemy_Red.png").as_slice(),
        ],
        "Scholar enemy atlas",
    )
}

fn load_recluse_atlas(render_context: &mut dyn RenderContext) -> Option<TextureId> {
    load_horizontal_atlas_256(
        render_context,
        &[
            include_bytes!("../assets/Recluse_Skill_Attribute_Frame.png").as_slice(),
            include_bytes!("../assets/Recluse_Skill_Attribute_Magic.png").as_slice(),
            include_bytes!("../assets/Recluse_Skill_Attribute_Fire.png").as_slice(),
            include_bytes!("../assets/Recluse_Skill_Attribute_Thunder.png").as_slice(),
            include_bytes!("../assets/Recluse_Skill_Attribute_Holy.png").as_slice(),
            include_bytes!("../assets/Recluse_Skill_Lock_Magic.png").as_slice(),
            include_bytes!("../assets/Recluse_Skill_Lock_Fire.png").as_slice(),
            include_bytes!("../assets/Recluse_Skill_Lock_Thunder.png").as_slice(),
            include_bytes!("../assets/Recluse_Skill_Lock_Holy.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic1.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic2.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic3.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic4.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic5.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic6.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic7.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic8.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic9.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic10.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic11.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic12.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic13.png").as_slice(),
            include_bytes!("../assets/Recluse_SkillMagic14.png").as_slice(),
        ],
        "Recluse atlas",
    )
}

fn load_revenant_summon_atlas(render_context: &mut dyn RenderContext) -> Option<TextureId> {
    let sources = [
        include_bytes!("../assets/Revenant_Skill_Summon_Frame.png").as_slice(),
        include_bytes!("../assets/Revenant_Skill_Summon_1.png").as_slice(),
        include_bytes!("../assets/Revenant_Skill_Summon_2.png").as_slice(),
        include_bytes!("../assets/Revenant_Skill_Summon_3.png").as_slice(),
    ];
    let mut atlas = image::RgbaImage::new(256 * sources.len() as u32, 512);
    for (index, bytes) in sources.into_iter().enumerate() {
        let image = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .expect("Revenant summon atlas format should be readable")
            .decode()
            .expect("Revenant summon atlas source should decode")
            .into_rgba8();
        image::imageops::overlay(&mut atlas, &image, (index as i64) * 256, 0);
    }
    let hp = ImageReader::new(Cursor::new(include_bytes!(
        "../assets/Revenant_Skill_Summon_HP.png"
    )))
    .with_guessed_format()
    .expect("Revenant summon HP format should be readable")
    .decode()
    .expect("Revenant summon HP should decode")
    .into_rgba8();
    image::imageops::overlay(&mut atlas, &hp, 0, 256);
    let width = atlas.width();
    let height = atlas.height();
    render_context.load_texture(atlas.as_raw(), width, height).ok()
}

fn load_horizontal_atlas_256(
    render_context: &mut dyn RenderContext,
    sources: &[&[u8]],
    label: &str,
) -> Option<TextureId> {
    let mut atlas = image::RgbaImage::new(256 * sources.len() as u32, 256);
    for (index, bytes) in sources.iter().copied().enumerate() {
        let image = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .unwrap_or_else(|_| panic!("{label} source format should be readable"))
            .decode()
            .unwrap_or_else(|_| panic!("{label} source should decode"))
            .into_rgba8();
        assert_eq!(image.width(), 256, "{label} source width should be 256");
        assert_eq!(image.height(), 256, "{label} source height should be 256");
        image::imageops::overlay(&mut atlas, &image, (index as i64) * 256, 0);
    }
    render_context
        .load_texture(atlas.as_raw(), atlas.width(), atlas.height())
        .ok()
}

fn load_horizontal_atlas_256_with_alpha(
    render_context: &mut dyn RenderContext,
    sources: &[(&[u8], f32)],
    label: &str,
) -> Option<TextureId> {
    let mut atlas = image::RgbaImage::new(256 * sources.len() as u32, 256);
    for (index, (bytes, alpha_multiplier)) in sources.iter().copied().enumerate() {
        let mut image = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .unwrap_or_else(|_| panic!("{label} source format should be readable"))
            .decode()
            .unwrap_or_else(|_| panic!("{label} source should decode"))
            .into_rgba8();
        assert_eq!(image.width(), 256, "{label} source width should be 256");
        assert_eq!(image.height(), 256, "{label} source height should be 256");
        if (alpha_multiplier - 1.0).abs() > f32::EPSILON {
            for alpha in image.as_mut().iter_mut().skip(3).step_by(4) {
                *alpha = ((*alpha as f32) * alpha_multiplier).clamp(0.0, 255.0) as u8;
            }
        }
        image::imageops::overlay(&mut atlas, &image, (index as i64) * 256, 0);
    }
    render_context
        .load_texture(atlas.as_raw(), atlas.width(), atlas.height())
        .ok()
}

fn crop_center_square(image: &image::RgbaImage, inset: u32) -> image::RgbaImage {
    let side = image.width().min(image.height());
    let safe_inset = inset.min((side - 1) / 2);
    let cropped_side = side - safe_inset * 2;
    let x = (image.width() - cropped_side) / 2;
    let y = (image.height() - cropped_side) / 2;
    image::imageops::crop_imm(image, x, y, cropped_side, cropped_side).to_image()
}

fn apply_circle_alpha_mask(rgba: &mut [u8], width: u32, height: u32) {
    let cx = (width as f32 - 1.0) * 0.5;
    let cy = (height as f32 - 1.0) * 0.5;
    let radius = width.min(height) as f32 * 0.515;
    let feather = width.min(height) as f32 * 0.015;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * width + x) * 4 + 3) as usize;

            if dist >= radius {
                rgba[idx] = 0;
            } else if dist > radius - feather {
                let fade = ((radius - dist) / feather).clamp(0.0, 1.0);
                rgba[idx] = (rgba[idx] as f32 * fade) as u8;
            }
        }
    }
}

const RECLUSE_ATLAS_SLOTS: usize = 23;
const RECLUSE_ATTRIBUTE_FRAME_ATLAS_SLOT: usize = 0;
const RECLUSE_ATTRIBUTE_ATLAS_FIRST_SLOT: usize = 1;
const RECLUSE_LOCK_ATLAS_FIRST_SLOT: usize = 5;
const RECLUSE_MAGIC_ATLAS_FIRST_SLOT: usize = 9;
const EFFECT_ATLAS_SLOTS: usize = 7;
const EFFECT_ULTIMATE_ON1_ATLAS_SLOT: usize = 0;
const EFFECT_ULTIMATE_ON2_ATLAS_SLOT: usize = 1;
const EFFECT_UNDERTAKER_SKILL_SP1_ATLAS_SLOT: usize = 2;
const EFFECT_UNDERTAKER_SKILL_SP2_ATLAS_SLOT: usize = 3;
const EFFECT_UNDERTAKER_ULTIMATE_SP1_ATLAS_SLOT: usize = 4;
const EFFECT_UNDERTAKER_ULTIMATE_SP2_ATLAS_SLOT: usize = 5;
const EFFECT_UNDERTAKER_ULTIMATE_SP3_ATLAS_SLOT: usize = 6;
