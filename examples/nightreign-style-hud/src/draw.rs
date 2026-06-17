fn draw_charge_slot(
    ui: &Ui,
    center: [f32; 2],
    radius: f32,
    slot: ChargeSlot,
    layers: [f32; 2],
    ready: bool,
    top_down_fill: bool,
    count: Option<usize>,
    icon_texture: Option<TextureRegion>,
    icon_texture_scale: f32,
    ready_texture: Option<TextureRegion>,
    active_textures: [Option<TextureRegion>; 3],
    suppress_ready_effect: bool,
    active_style: ActiveTextureStyle,
    now: Instant,
    animation_start: Instant,
) {
    let shadow = ImColor32::from_rgba(0, 0, 0, 95);
    let plate = ImColor32::from_rgba(18, 24, 36, 170);
    let border = if ready {
        ImColor32::from_rgba(208, 225, 255, 210)
    } else {
        ImColor32::from_rgba(184, 194, 215, 155)
    };

    {
        let draw = ui.get_foreground_draw_list();
        draw.add_circle(center, radius + 4.0, shadow)
            .num_segments(96)
            .filled(true)
            .build();
        draw.add_circle(center, radius, plate)
            .num_segments(96)
            .filled(true)
            .build();
    }

    draw_png_icon(ui, center, radius, icon_texture, icon_texture_scale, ready);
    if top_down_fill {
        draw_bottom_fill(ui, center, radius - 3.0, layers[0], slot.fill);
    } else if !ready {
        draw_bottom_fill(ui, center, radius - 3.0, layers[0], slot.fill);
    } else if count.is_some() && layers[1] > 0.0 && layers[1] < 1.0 {
        draw_bottom_fill(
            ui,
            center,
            radius - 8.0 * self_scale(radius),
            layers[1],
            ImColor32::from_rgba(145, 196, 255, 112),
        );
    }
    if icon_texture.is_none() {
        draw_icon_mark(ui, center, radius, slot.label, slot.accent, slot.icon_scale);
    }

    {
        let draw = ui.get_foreground_draw_list();
        draw.add_circle(center, radius, border)
            .num_segments(96)
            .thickness(1.5 * self_scale(radius))
            .build();
    }

    if ready && !suppress_ready_effect {
        if let Some(texture) = ready_texture {
            draw_ready_texture(ui, center, radius, texture, now, animation_start);
        } else {
            draw_ready_effect(ui, center, radius, slot.ready_glow, now, animation_start);
        }
    }

    for (index, texture) in active_textures.into_iter().enumerate() {
        if let Some(texture) = texture {
            draw_active_texture(
                ui,
                center,
                radius,
                texture,
                index,
                active_style,
                now,
                animation_start,
            );
        }
    }

    if let Some(count) = count {
        draw_charge_count(ui, center, radius, count);
    }
}

fn draw_recluse_skill_ui(
    ui: &Ui,
    center: [f32; 2],
    radius: f32,
    snapshot: &HudSnapshot,
    icons: &RecluseIconSet,
) {
    let slot_half = radius * 1.38;
    let slot_positions = [
        [center[0], center[1] - radius * 1.08],
        [center[0] + radius * 0.9, center[1] + radius * 0.74],
        [center[0] - radius * 0.9, center[1] + radius * 0.74],
    ];

    for (index, slot_center) in slot_positions.into_iter().enumerate() {
        if let Some(texture_id) = icons.atlas {
            let region = TextureRegion::atlas_slot(
                texture_id,
                RECLUSE_ATTRIBUTE_FRAME_ATLAS_SLOT,
                RECLUSE_ATLAS_SLOTS,
            );
            draw_centered_texture_region(
                ui,
                slot_center,
                slot_half,
                region,
                ImColor32::from_rgba(255, 255, 255, 165),
            );
        }
        if let Some(element) = snapshot.recluse_elements[index] {
            if let Some(texture_id) = icons.atlas {
                let region = TextureRegion::atlas_slot(
                    texture_id,
                    RECLUSE_ATTRIBUTE_ATLAS_FIRST_SLOT + element.icon_index(),
                    RECLUSE_ATLAS_SLOTS,
                );
                draw_centered_texture_region(
                    ui,
                    slot_center,
                    slot_half * 0.88,
                    region,
                    ImColor32::from_rgba(255, 255, 255, 235),
                );
            }
        }
    }

    if let Some(element) = snapshot.recluse_lock_element {
        let Some(pos) = snapshot.recluse_lock_pos else {
            return;
        };
        if let Some(texture_id) = icons.atlas {
            let region = TextureRegion::atlas_slot(
                texture_id,
                RECLUSE_LOCK_ATLAS_FIRST_SLOT + element.icon_index(),
                RECLUSE_ATLAS_SLOTS,
            );
            draw_centered_texture_region(
                ui,
                pos,
                radius * 3.68,
                region,
                ImColor32::from_rgba(255, 255, 255, 245),
            );
        }
        if RECLUSE_LOCK_DEBUG_POINTS {
            draw_recluse_lock_debug_points(ui, snapshot.recluse_lock_debug_points);
        }
    }
}

fn draw_recluse_lock_debug_points(ui: &Ui, points: LockDebugPoints) {
    let labels = ["A", "B", "C", "D", "E"];
    let colors = [
        ImColor32::from_rgba(255, 80, 80, 255),
        ImColor32::from_rgba(80, 180, 255, 255),
        ImColor32::from_rgba(120, 255, 120, 255),
        ImColor32::from_rgba(255, 220, 80, 255),
        ImColor32::from_rgba(235, 120, 255, 255),
    ];
    let draw = ui.get_foreground_draw_list();
    for (index, point) in points.into_iter().enumerate() {
        let Some(pos) = point else {
            continue;
        };
        draw.add_circle(pos, 11.0, ImColor32::from_rgba(0, 0, 0, 220))
            .thickness(4.0)
            .num_segments(24)
            .build();
        draw.add_circle(pos, 10.0, colors[index])
            .thickness(3.0)
            .num_segments(24)
            .build();
        draw.add_circle(pos, 3.0, ImColor32::from_rgba(255, 255, 255, 255))
            .filled(true)
            .num_segments(16)
            .build();
        draw_sized_text(
            [pos[0] + 13.0, pos[1] - 17.0],
            22.0,
            colors[index],
            labels[index],
        );
    }
}

fn draw_revenant_static_summon_ui(
    ui: &Ui,
    skill_center: [f32; 2],
    skill_radius: f32,
    snapshot: &HudSnapshot,
    icons: &RevenantIconSet,
    scale: f32,
) {
    let draw = ui.get_foreground_draw_list();
    let frame_h = 70.0 * scale;
    let row_step = 55.0 * scale;
    let frame_w = 114.4 * scale;
    let base_x =
        snap_px((skill_center[0] - skill_radius - frame_w - 178.0 * scale).max(36.0 * scale));
    let base_y = snap_px(skill_center[1] - row_step - frame_h * 0.5);

    for index in 0..3 {
        let summon = snapshot.revenant_summons[index];
        let y = snap_px(base_y + index as f32 * row_step);
        let row_min = [base_x, y];
        let row_max = [snap_px(base_x + frame_w), snap_px(y + frame_h)];

        if let Some(texture_id) = icons.summon_atlas {
            draw.add_image(texture_id, row_min, row_max)
                .uv_min(REVENANT_SUMMON_FRAME_UV_MIN)
                .uv_max(REVENANT_SUMMON_FRAME_UV_MAX)
                .col(if summon.active {
                    ImColor32::from_rgba(255, 255, 255, 255)
                } else {
                    ImColor32::from_rgba(255, 255, 255, 205)
                })
                .build();
        } else {
            draw.add_rect(row_min, row_max, ImColor32::from_rgba(168, 190, 235, 180))
                .thickness(1.0 * scale)
                .build();
        }

        let icon_half = frame_h * 0.51;
        let icon_center = [
            snap_px(base_x + frame_h * 0.38),
            snap_px(y + frame_h * 0.57),
        ];
        if let Some(texture_id) = icons.summon_atlas {
            let uv_min_x = 0.25 * (index as f32 + 1.0);
            let uv_max_x = uv_min_x + 0.25;
            draw.add_image(
                texture_id,
                [icon_center[0] - icon_half, icon_center[1] - icon_half],
                [icon_center[0] + icon_half, icon_center[1] + icon_half],
            )
            .uv_min([uv_min_x, 0.0])
            .uv_max([uv_max_x, REVENANT_SUMMON_TOP_UV_MAX_Y])
            .col(if summon.active {
                ImColor32::from_rgba(255, 255, 255, 255)
            } else {
                ImColor32::from_rgba(118, 122, 134, 180)
            })
            .build();
        } else {
            draw.add_circle(icon_center, icon_half * 0.74, ImColor32::from_rgba(188, 198, 224, 120))
                .num_segments(32)
                .filled(true)
                .build();
            draw.add_circle(icon_center, icon_half * 0.74, ImColor32::from_rgba(218, 229, 255, 175))
                .num_segments(32)
                .thickness(1.2 * scale)
                .build();
        }

        let bar_x = base_x + 69.0 * scale;
        let bar_w = 152.0 * scale;
        let bar_h = 10.5 * scale;
        let bar_y = snap_px(y + frame_h * 0.45 - bar_h * 0.5);
        let bar_x = snap_px(bar_x);
        let hp_ratio = summon.hp_ratio.clamp(0.0, 1.0);
        if let Some(texture_id) = icons.summon_atlas {
            if hp_ratio > 0.0 {
                let hp_uv_max = [
                    REVENANT_SUMMON_HP_UV_MIN[0]
                        + (REVENANT_SUMMON_HP_UV_MAX[0] - REVENANT_SUMMON_HP_UV_MIN[0])
                            * hp_ratio,
                    REVENANT_SUMMON_HP_UV_MAX[1],
                ];
                draw.add_image(
                    texture_id,
                    [bar_x, bar_y],
                    [snap_px(bar_x + bar_w * hp_ratio), snap_px(bar_y + bar_h)],
                )
                .uv_min(REVENANT_SUMMON_HP_UV_MIN)
                .uv_max(hp_uv_max)
                .col(ImColor32::from_rgba(255, 255, 255, 230))
                .build();
            }
        } else {
            draw.add_rect(
                [bar_x, bar_y],
                [bar_x + bar_w, bar_y + bar_h],
                ImColor32::from_rgba(188, 160, 226, 220),
            )
            .thickness(1.0 * scale)
            .build();
        }
    }

    if REVENANT_SUMMON_DEBUG_VISIBLE {
        let active = snapshot
            .revenant_active_summon
            .map(|index| (index + 1).to_string())
            .unwrap_or_else(|| "-".to_string());
        let handle = if snapshot.revenant_active_handle_index == u32::MAX {
            "-".to_string()
        } else {
            format!(
                "{}:{}",
                snapshot.revenant_active_handle_container, snapshot.revenant_active_handle_index
            )
        };
        let text = format!(
            "Revenant summon mask:{:03b} active:{} buddies:{} handle:{} npc:{} hp:{:.2}/{:.2}/{:.2}",
            snapshot.revenant_effect_mask,
            active,
            snapshot.revenant_buddy_count,
            handle,
            snapshot.revenant_active_npc_id,
            snapshot.revenant_summons[0].hp_ratio,
            snapshot.revenant_summons[1].hp_ratio,
            snapshot.revenant_summons[2].hp_ratio,
        );
        draw_sized_text(
            [base_x, base_y - 22.0 * scale],
            16.0 * scale,
            ImColor32::from_rgba(220, 240, 255, 230),
            &text,
        );
    }

    if REVENANT_GAME_SETTINGS_DEBUG_VISIBLE {
        draw_revenant_game_settings_debug(base_x, base_y - 74.0 * scale, 14.0 * scale);
    }
}

fn draw_revenant_game_settings_debug(x: f32, y: f32, font_size: f32) {
    let Some(lines) = revenant_game_settings_debug_lines() else {
        return;
    };
    for (index, line) in lines.iter().enumerate() {
        draw_sized_text(
            [x, y + index as f32 * (font_size + 2.0)],
            font_size,
            ImColor32::from_rgba(230, 245, 255, 235),
            line,
        );
    }
}

fn revenant_game_settings_debug_lines() -> Option<[String; 3]> {
    const BYTES: usize = REVENANT_GAME_SETTINGS_DEBUG_BYTES;
    let game_data_man = unsafe { GameDataMan::instance() }.ok()?;
    let settings = game_data_man.game_settings.as_ref();
    let ptr = settings as *const _ as *const u8;
    let mut current = [0u8; BYTES];
    unsafe {
        std::ptr::copy_nonoverlapping(ptr, current.as_mut_ptr(), BYTES);
    }

    let mut baseline = REVENANT_GAME_SETTINGS_BASELINE.lock().ok()?;
    let baseline = baseline.get_or_insert(current);
    let mut diffs = Vec::new();
    for (offset, (old, new)) in baseline.iter().zip(current.iter()).enumerate() {
        if old != new {
            diffs.push(format!("{offset:03X}:{old:02X}>{new:02X}"));
        }
    }
    if diffs.len() > 18 {
        diffs.truncate(18);
        diffs.push("...".to_string());
    }

    let hud_type = current.get(9).copied().unwrap_or_default();
    let changed = if diffs.is_empty() {
        "-".to_string()
    } else {
        diffs.join(" ")
    };
    let page0 = format_settings_bytes("000", &current[0x00..0x10]);
    let page1 = format_settings_bytes("0A0", &current[0xA0..0xB0]);
    Some([
        format!("GameSettings hud_type:{hud_type} changed:{changed}"),
        page0,
        page1,
    ])
}

fn format_settings_bytes(label: &str, bytes: &[u8]) -> String {
    let mut text = format!("{label}:");
    for byte in bytes {
        text.push_str(&format!(" {byte:02X}"));
    }
    text
}

fn draw_ironeye_weakness_marks(
    ui: &Ui,
    snapshot: &HudSnapshot,
    icons: &IroneyeIconSet,
    now: Instant,
    animation_start: Instant,
) {
    for mark in snapshot.ironeye_weakness_marks {
        if !mark.active {
            continue;
        }
        let progress = mark.progress.clamp(0.0, 1.0);
        let red = lerp(150.0, 255.0, progress) as u8;
        let green = lerp(74.0, 18.0, progress) as u8;
        let blue = lerp(62.0, 28.0, progress) as u8;
        let mut color = ImColor32::from_rgba(red, green, blue, 225);
        if mark.remaining <= IRONEYE_WEAKNESS_FLASH_SECONDS {
            let elapsed = (now - animation_start).as_secs_f32();
            let flash = ((elapsed * 16.0).sin() * 0.5 + 0.5)
                * (1.0 - mark.remaining / IRONEYE_WEAKNESS_FLASH_SECONDS).clamp(0.0, 1.0);
            if flash > 0.35 {
                let alpha = lerp(150.0, 245.0, flash) as u8;
                color = ImColor32::from_rgba(255, 246, 228, alpha);
            }
        }
        let half = lerp(28.0, 36.0, progress) * IRONEYE_WEAKNESS_MARK_SCALE;
        if let Some(texture_id) = icons.atlas {
            for slot in IRONEYE_WEAKNESS_ATLAS_SLOTS {
                let (uv_min, uv_max) = atlas_uv(slot, IRONEYE_ATLAS_SLOTS);
                draw_centered_atlas_texture(ui, mark.pos, half, texture_id, uv_min, uv_max, color);
            }
        } else {
            let draw = ui.get_foreground_draw_list();
            draw.add_circle(mark.pos, half * 0.66, color)
                .num_segments(32)
                .thickness(3.0)
                .build();
        }
    }

    for burst in snapshot.ironeye_weakness_bursts {
        if !burst.active {
            continue;
        }
        let spread_t = (burst.age / IRONEYE_WEAKNESS_BURST_SPREAD_SECONDS).clamp(0.0, 1.0);
        let fade_start = IRONEYE_WEAKNESS_BURST_SPREAD_SECONDS + IRONEYE_WEAKNESS_BURST_HOLD_SECONDS;
        let fade_t = ((burst.age - fade_start) / IRONEYE_WEAKNESS_BURST_FADE_SECONDS)
            .clamp(0.0, 1.0);
        let alpha = (245.0 * (1.0 - fade_t).powf(1.35)) as u8;
        if alpha == 0 {
            continue;
        }
        let half = lerp(36.0, 44.0, spread_t) * IRONEYE_WEAKNESS_MARK_SCALE;
        let color = ImColor32::from_rgba(255, 244, 230, alpha);
        if let Some(texture_id) = icons.atlas {
            let directions = [[-0.72, -0.54], [0.72, -0.54], [0.72, 0.54], [-0.72, 0.54]];
            let spread = lerp(0.0, 28.0, spread_t.powf(0.72)) * IRONEYE_WEAKNESS_MARK_SCALE;
            for (index, slot) in IRONEYE_WEAKNESS_ATLAS_SLOTS.into_iter().enumerate() {
                let dir = directions[index];
                let pos = [
                    burst.pos[0] + dir[0] * spread,
                    burst.pos[1] + dir[1] * spread,
                ];
                let (uv_min, uv_max) = atlas_uv(slot, IRONEYE_ATLAS_SLOTS);
                draw_centered_atlas_texture(ui, pos, half, texture_id, uv_min, uv_max, color);
            }
        } else {
            ui.get_foreground_draw_list()
                .add_circle(burst.pos, half * 0.66, color)
                .num_segments(48)
                .thickness(4.0)
                .build();
        }
    }
}

fn draw_ironeye_precision_reticle(
    ui: &Ui,
    snapshot: &HudSnapshot,
    icons: &IroneyeIconSet,
    scale: f32,
) {
    if !snapshot.ironeye_precision_aiming {
        return;
    }

    let viewport = ui.io().display_size;
    let center = ironeye_precision_reticle_screen_pos(viewport);
    let mut drew_reticle = false;
    if let Some(texture_id) = icons.atlas {
        for slot in IRONEYE_RETICLE_ATLAS_SLOTS {
            let (uv_min, uv_max) = atlas_uv(slot, IRONEYE_ATLAS_SLOTS);
            draw_centered_atlas_texture(
                ui,
                center,
                IRONEYE_RETICLE_HALF_SIZE * scale,
                texture_id,
                uv_min,
                uv_max,
                ImColor32::from_rgba(255, 255, 255, 235),
            );
            drew_reticle = true;
        }
    }
    if !drew_reticle {
        let draw = ui.get_foreground_draw_list();
        let half = IRONEYE_RETICLE_HALF_SIZE * 0.42 * scale;
        let color = ImColor32::from_rgba(244, 247, 255, 220);
        draw.add_line([center[0] - half, center[1]], [center[0] + half, center[1]], color)
            .thickness(1.5 * scale)
            .build();
        draw.add_line([center[0], center[1] - half], [center[0], center[1] + half], color)
            .thickness(1.5 * scale)
            .build();
        draw.add_circle(center, half * 0.36, color)
            .num_segments(48)
            .thickness(1.4 * scale)
            .build();
    }
}

#[allow(dead_code)]
fn draw_ironeye_camera_debug() {
    let Some(lines) = ironeye_camera_debug_lines() else {
        return;
    };
    for (index, line) in lines.iter().enumerate() {
        draw_sized_text(
            [24.0, 210.0 + index as f32 * 20.0],
            16.0,
            ImColor32::from_rgba(235, 245, 255, 220),
            line,
        );
    }
}

fn draw_scholar_observation_ui(
    ui: &Ui,
    snapshot: &HudSnapshot,
    icons: &ScholarIconSet,
    _now: Instant,
    _animation_start: Instant,
) {
    if !snapshot.scholar_observing {
        return;
    }

    let viewport = ui.io().display_size;
    let center = [
        viewport[0] * SCHOLAR_LENS_CENTER_X_FACTOR,
        viewport[1] * 0.5,
    ];
    let lens_half = viewport[1].min(viewport[0]) * SCHOLAR_LENS_RADIUS_FACTOR;
    if let Some(texture_id) = icons.lens {
        draw_centered_texture(
            ui,
            center,
            lens_half,
            texture_id,
            ImColor32::from_rgba(255, 255, 255, 205),
        );
    } else {
        ui.get_foreground_draw_list()
            .add_circle(center, lens_half, ImColor32::from_rgba(220, 235, 255, 120))
            .num_segments(160)
            .thickness(3.0)
            .build();
    }

    for target in snapshot.scholar_targets {
        if !target.active || target.progress <= 0.0 {
            continue;
        }
        draw_scholar_target_marker(ui, target, icons);
    }

    if SCHOLAR_SCAN_DEBUG_VISIBLE {
        draw_scholar_debug(snapshot.scholar_debug);
    }
}

#[allow(dead_code)]
fn draw_multi_locking_targets(
    ui: &Ui,
    _active: bool,
    targets: [MultiLockTargetSnapshot; MULTI_LOCK_BULLET_MAX_TARGETS],
    texture_id: Option<TextureId>,
) {
    for target in targets {
        if !target.active {
            continue;
        }
        let half = if target.primary { 68.0 } else { 58.0 };
        let color = if target.primary {
            ImColor32::from_rgba(255, 255, 255, 235)
        } else {
            ImColor32::from_rgba(205, 225, 255, 205)
        };
        if let Some(texture_id) = texture_id {
            draw_centered_texture(ui, target.pos, half, texture_id, color);
        } else {
            ui.get_foreground_draw_list()
                .add_circle(target.pos, half, color)
                .num_segments(72)
                .thickness(if target.primary { 3.0 } else { 2.0 })
                .build();
        }
    }
}

fn draw_scholar_damage_numbers(_ui: &Ui, snapshot: &HudSnapshot) {
    for number in snapshot.scholar_damage_numbers {
        if !number.active || number.amount <= 0 {
            continue;
        }
        let t = (number.age / SCHOLAR_DAMAGE_NUMBER_SECONDS).clamp(0.0, 1.0);
        let alpha = lerp(255.0, 0.0, t * t) as u8;
        if alpha == 0 {
            continue;
        }
        let font_size = lerp(31.0, 23.0, t);
        let text = number.amount.to_string();
        let text_width = text.len() as f32 * font_size * 0.54;
        let drift_x = (number.seed - 2.0) * 11.0 * t;
        let pos = [
            number.pos[0] - text_width * 0.5 + drift_x,
            number.pos[1] - 62.0 - 44.0 * t,
        ];
        let shadow = ImColor32::from_rgba(18, 8, 4, (alpha as f32 * 0.72) as u8);
        for offset in [[2.0, 2.0], [-1.0, 2.0], [2.0, -1.0]] {
            draw_sized_text(
                [pos[0] + offset[0], pos[1] + offset[1]],
                font_size,
                shadow,
                &text,
            );
        }
        draw_sized_text(
            pos,
            font_size,
            ImColor32::from_rgba(255, 224, 150, alpha),
            &text,
        );
    }
}

fn draw_scholar_debug(debug: ScholarScanDebug) {
    let text = format!(
        "Scholar scan  all:{} type:{} hidden:{} hp:{} screen:{} range:{} los:{} ok:{}",
        debug.scanned,
        debug.type_skip,
        debug.hidden_skip,
        debug.hp_skip,
        debug.screen_skip,
        debug.range_skip,
        debug.los_skip,
        debug.accepted
    );
    draw_sized_text(
        [24.0, 210.0],
        18.0,
        ImColor32::from_rgba(235, 245, 255, 220),
        &text,
    );
    let candidate = debug.candidate;
    if !candidate.active {
        return;
    }
    let id_text = format!(
        "Scholar cand  npc:{} type:{} hp:{}/{} dist:{:.1} h:{}:{}:{}:{} sel:{}:{} load:{} upd:{}",
        candidate.npc_id,
        candidate.chr_type,
        candidate.hp,
        candidate.max_hp,
        candidate.distance,
        candidate.block_area,
        candidate.block_block,
        candidate.block_region,
        candidate.block_index,
        candidate.selector_container,
        candidate.selector_index,
        candidate.load_status,
        candidate.chr_update_type,
    );
    let flag_text = format!(
        "load dg:{} br:{} host:{} ext:{} near:{} | render rg:{} on:{} en:{} dead:{}",
        candidate.draw_group as u8,
        candidate.backread_disabled as u8,
        candidate.host_inactive as u8,
        candidate.extinction_death as u8,
        candidate.near_pc as u8,
        candidate.render_group as u8,
        candidate.onscreen_flag as u8,
        candidate.enable_render as u8,
        candidate.death_flag as u8,
    );
    let state_text = format!(
        "state active:{} task:{} unload:{} disabled:{} tint:{:.2}/{:.2} base:{:.2}/{:.2}",
        candidate.is_active as u8,
        candidate.update_tasks_registered as u8,
        candidate.force_unloaded as u8,
        candidate.character_disabled as u8,
        candidate.tint_alpha,
        candidate.tint_alpha_modifier,
        candidate.base_transparency,
        candidate.base_transparency_modifier,
    );
    let event_text = format!(
        "event id:{} ef:{:02X} act:{} th:{} snd:{} anim:{}/{}/{} tae:{} op:{:.2}/{:.2} camo:{:.2} d:{:.0}/{:.0} mr:{:.1} at:{:.1}",
        candidate.event_entity_id,
        candidate.chr_set_entry_flags,
        candidate.activation_enabled as u8,
        candidate.activate_threshold_exceeded as u8,
        candidate.sounds_active as u8,
        candidate.current_anim_id,
        candidate.request_anim_id,
        candidate.idle_anim_id,
        candidate.current_tae_id,
        candidate.opacity_keyframes,
        candidate.opacity_keyframes_previous,
        candidate.camouflage_transparency,
        candidate.distance_to_player_sqr,
        candidate.horizontal_distance_to_player_sqr,
        candidate.max_render_range,
        candidate.chr_activate_threshold,
    );
    let los_text = format!(
        "los f[0,1,2,4,8,16,32,64,128] hit:{:03X} near:{:03X} block:{:03X} closest:{:.1} target:{:.1} bf:{}",
        candidate.los_hit_mask,
        candidate.los_near_mask,
        candidate.los_block_mask,
        candidate.los_closest_hit_distance,
        candidate.los_target_distance,
        candidate.los_block_filter,
    );
    let ctrl_text = format!(
        "ctrl coll:{:X} mv:{} pc:{} hit:{} map:{} cap:{} obj:{} sync:{}/{}",
        candidate.chr_collision,
        candidate.ctrl_disable_move as u8,
        candidate.ctrl_disable_player_collision as u8,
        candidate.ctrl_disable_hit as u8,
        candidate.ctrl_disable_map_collision as u8,
        candidate.ctrl_disable_capsule_collision as u8,
        candidate.ctrl_disable_object_collision as u8,
        candidate.ctrl_proxy_pos_sync as u8,
        candidate.ctrl_proxy_rot_sync as u8,
    );
    let physics_text = format!(
        "phys proxy:{:X}/{:X} shape:{:X} posreq:{} ground:{}/{} slide:{} grav:{}",
        candidate.physics_chr_proxy,
        candidate.physics_chr_proxy2,
        candidate.physics_collision_shape,
        candidate.physics_pos_update_requested as u8,
        candidate.physics_standing_on_ground as u8,
        candidate.physics_touching_ground as u8,
        candidate.physics_slide_enabled as u8,
        candidate.physics_gravity_disabled as u8,
    );
    let raw_ptr_text = format!(
        "raw ptr chr:{:X} data:{:X} entry:{:X} msbdf:{:08X} part:{:X}",
        candidate.chr_ptr,
        candidate.data_ptr,
        candidate.chr_set_entry_ptr,
        candidate.msb_draw_flags,
        candidate.msb_part_ptr,
    );
    let raw_hash_text = format!(
        "raw hash chr:{:08X} data:{:08X} msb:{:08X} entry:{:08X}",
        candidate.chr_hash,
        candidate.data_hash,
        candidate.msb_hash,
        candidate.entry_hash,
    );
    let entry_raw_text = format!(
        "entry raw {:016X} {:016X}",
        candidate.entry_raw0,
        candidate.entry_raw1,
    );
    draw_sized_text(
        [24.0, 232.0],
        16.0,
        ImColor32::from_rgba(220, 245, 255, 220),
        &id_text,
    );
    draw_sized_text(
        [24.0, 252.0],
        16.0,
        ImColor32::from_rgba(220, 245, 255, 220),
        &flag_text,
    );
    draw_sized_text(
        [24.0, 272.0],
        16.0,
        ImColor32::from_rgba(220, 245, 255, 220),
        &state_text,
    );
    draw_sized_text(
        [24.0, 292.0],
        16.0,
        ImColor32::from_rgba(220, 245, 255, 220),
        &event_text,
    );
    draw_sized_text(
        [24.0, 312.0],
        16.0,
        ImColor32::from_rgba(210, 235, 255, 225),
        &ctrl_text,
    );
    draw_sized_text(
        [24.0, 332.0],
        16.0,
        ImColor32::from_rgba(210, 235, 255, 225),
        &physics_text,
    );
    draw_sized_text(
        [24.0, 352.0],
        16.0,
        ImColor32::from_rgba(210, 235, 255, 225),
        &raw_ptr_text,
    );
    draw_sized_text(
        [24.0, 372.0],
        16.0,
        ImColor32::from_rgba(210, 235, 255, 225),
        &raw_hash_text,
    );
    draw_sized_text(
        [24.0, 392.0],
        16.0,
        ImColor32::from_rgba(210, 235, 255, 225),
        &entry_raw_text,
    );
    draw_sized_text(
        [24.0, 412.0],
        16.0,
        ImColor32::from_rgba(255, 230, 190, 230),
        &los_text,
    );
}

fn draw_scholar_target_marker(ui: &Ui, target: ScholarTargetSnapshot, icons: &ScholarIconSet) {
    let progress = target.progress.clamp(0.0, 1.0);
    let half = 62.0;
    let (red_color, pointer_color) = if target.observed {
        scholar_marker_progress_colors(progress)
    } else {
        scholar_marker_decay_colors(progress)
    };
    if let Some(texture_id) = icons.enemy_atlas {
        let (uv_min, uv_max) =
            atlas_uv(SCHOLAR_ENEMY_RED_ATLAS_SLOT, SCHOLAR_ENEMY_ATLAS_SLOTS);
        draw_textured_progress_sector_atlas(
            ui,
            target.pos,
            half,
            texture_id,
            uv_min,
            uv_max,
            progress,
            red_color,
        );
    }
    if let Some(texture_id) = icons.enemy_atlas {
        let (uv_min, uv_max) =
            atlas_uv(SCHOLAR_ENEMY_WHITE_ATLAS_SLOT, SCHOLAR_ENEMY_ATLAS_SLOTS);
        draw_centered_atlas_texture(
            ui,
            target.pos,
            half * 1.03,
            texture_id,
            uv_min,
            uv_max,
            ImColor32::from_rgba(255, 255, 255, 220),
        );
    }

    let angle = progress * std::f32::consts::TAU;
    if target.observed {
        if let Some(texture_id) = icons.enemy_atlas {
            let (uv_min, uv_max) =
                atlas_uv(SCHOLAR_ENEMY_POINTER_ATLAS_SLOT, SCHOLAR_ENEMY_ATLAS_SLOTS);
            draw_rotated_atlas_texture(
                ui,
                target.pos,
                half * 1.02,
                texture_id,
                uv_min,
                uv_max,
                angle,
                pointer_color,
            );
        }
    } else {
        if let Some(texture_id) = icons.enemy_atlas {
            let (uv_min, uv_max) =
                atlas_uv(SCHOLAR_ENEMY_POINTER_BACK_ATLAS_SLOT, SCHOLAR_ENEMY_ATLAS_SLOTS);
            draw_rotated_atlas_texture(
                ui,
                target.pos,
                half * 1.02,
                texture_id,
                uv_min,
                uv_max,
                angle,
                pointer_color,
            );
        } else {
            draw_scholar_decay_pointer(ui, target.pos, half, angle, pointer_color);
        }
    }

    if target.observed && progress >= 1.0 {
        if let Some(texture_id) = icons.enemy_atlas {
            let (uv_min, uv_max) =
                atlas_uv(SCHOLAR_ENEMY_FULL_ATLAS_SLOT, SCHOLAR_ENEMY_ATLAS_SLOTS);
            draw_centered_atlas_texture(
                ui,
                target.pos,
                half * 1.08,
                texture_id,
                uv_min,
                uv_max,
                ImColor32::from_rgba(255, 255, 255, 245),
            );
        }
    }
}

fn scholar_marker_decay_colors(progress: f32) -> (ImColor32, ImColor32) {
    let alpha = lerp(188.0, 238.0, progress) as u8;
    (
        ImColor32::from_rgba(32, 36, 40, alpha),
        ImColor32::from_rgba(210, 216, 220, lerp(205.0, 245.0, progress) as u8),
    )
}

fn scholar_marker_progress_colors(progress: f32) -> (ImColor32, ImColor32) {
    if progress >= 0.995 {
        return (
            ImColor32::from_rgba(255, 96, 78, 245),
            ImColor32::from_rgba(255, 116, 96, 235),
        );
    }

    if progress < 0.5 {
        let t = progress / 0.5;
        let red_alpha = lerp(118.0, 172.0, t) as u8;
        let pointer_alpha = lerp(118.0, 158.0, t) as u8;
        return (
            ImColor32::from_rgba(150, 46, 42, red_alpha),
            ImColor32::from_rgba(170, 70, 60, pointer_alpha),
        );
    }

    let t = (progress - 0.5) / 0.5;
    (
        ImColor32::from_rgba(
            lerp(205.0, 255.0, t) as u8,
            lerp(98.0, 255.0, t) as u8,
            lerp(86.0, 255.0, t) as u8,
            lerp(185.0, 210.0, t) as u8,
        ),
        ImColor32::from_rgba(
            255,
            lerp(145.0, 220.0, t) as u8,
            lerp(126.0, 210.0, t) as u8,
            lerp(165.0, 185.0, t) as u8,
        ),
    )
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t.clamp(0.0, 1.0)
}

fn draw_textured_progress_sector_atlas(
    ui: &Ui,
    center: [f32; 2],
    radius: f32,
    texture_id: TextureId,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    progress: f32,
    color: ImColor32,
) {
    if progress <= 0.0 {
        return;
    }
    if progress >= 0.995 {
        draw_centered_atlas_texture(ui, center, radius, texture_id, uv_min, uv_max, color);
        return;
    }

    let draw = ui.get_foreground_draw_list();
    let angle = progress.clamp(0.0, 1.0) * std::f32::consts::TAU;
    let steps = ((angle / std::f32::consts::TAU) * 48.0).ceil().max(1.0) as usize;
    let center_uv = atlas_local_uv([0.5, 0.5], uv_min, uv_max);
    let point = |theta: f32| -> ([f32; 2], [f32; 2]) {
        let x = theta.sin() * radius;
        let y = -theta.cos() * radius;
        (
            [center[0] + x, center[1] + y],
            atlas_local_uv([(x / radius + 1.0) * 0.5, (y / radius + 1.0) * 0.5], uv_min, uv_max),
        )
    };

    let mut previous = point(0.0);
    for step in 1..=steps {
        let theta = angle * step as f32 / steps as f32;
        let current = point(theta);
        draw.add_image_quad(texture_id, center, previous.0, current.0, center)
            .uv(center_uv, previous.1, current.1, center_uv)
            .col(color)
            .build();
        previous = current;
    }
}

fn draw_rotated_atlas_texture(
    ui: &Ui,
    center: [f32; 2],
    half: f32,
    texture_id: TextureId,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    angle: f32,
    color: ImColor32,
) {
    let cos = angle.cos();
    let sin = angle.sin();
    let rotate = |x: f32, y: f32| -> [f32; 2] {
        [center[0] + x * cos - y * sin, center[1] + x * sin + y * cos]
    };
    ui.get_foreground_draw_list()
        .add_image_quad(
            texture_id,
            rotate(-half, -half),
            rotate(half, -half),
            rotate(half, half),
            rotate(-half, half),
        )
        .uv(uv_min, [uv_max[0], uv_min[1]], uv_max, [uv_min[0], uv_max[1]])
        .col(color)
        .build();
}

fn draw_scholar_decay_pointer(
    ui: &Ui,
    center: [f32; 2],
    radius: f32,
    angle: f32,
    color: ImColor32,
) {
    let dir = [angle.sin(), -angle.cos()];
    let start = [
        center[0] + dir[0] * radius * 0.12,
        center[1] + dir[1] * radius * 0.12,
    ];
    let end = [
        center[0] + dir[0] * radius * 0.88,
        center[1] + dir[1] * radius * 0.88,
    ];
    let draw = ui.get_foreground_draw_list();
    draw.add_line(start, end, ImColor32::from_rgba(20, 24, 28, 190))
        .thickness(5.0)
        .build();
    draw.add_line(start, end, color).thickness(2.7).build();
}

fn draw_centered_texture(
    ui: &Ui,
    center: [f32; 2],
    half: f32,
    texture_id: TextureId,
    color: ImColor32,
) {
    ui.get_foreground_draw_list()
        .add_image(
            texture_id,
            [center[0] - half, center[1] - half],
            [center[0] + half, center[1] + half],
        )
        .col(color)
        .build();
}

fn draw_centered_atlas_texture(
    ui: &Ui,
    center: [f32; 2],
    half: f32,
    texture_id: TextureId,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    color: ImColor32,
) {
    ui.get_foreground_draw_list()
        .add_image(
            texture_id,
            [center[0] - half, center[1] - half],
            [center[0] + half, center[1] + half],
        )
        .uv_min(uv_min)
        .uv_max(uv_max)
        .col(color)
        .build();
}

fn draw_centered_texture_region(
    ui: &Ui,
    center: [f32; 2],
    half: f32,
    texture: TextureRegion,
    color: ImColor32,
) {
    ui.get_foreground_draw_list()
        .add_image(
            texture.texture_id,
            [center[0] - half, center[1] - half],
            [center[0] + half, center[1] + half],
        )
        .uv_min(texture.uv_min)
        .uv_max(texture.uv_max)
        .col(color)
        .build();
}

fn atlas_uv(slot: usize, total_slots: usize) -> ([f32; 2], [f32; 2]) {
    let slot_w = 1.0 / total_slots as f32;
    let min_x = slot as f32 * slot_w;
    ([min_x, 0.0], [min_x + slot_w, 1.0])
}

fn atlas_local_uv(local: [f32; 2], uv_min: [f32; 2], uv_max: [f32; 2]) -> [f32; 2] {
    [
        uv_min[0] + (uv_max[0] - uv_min[0]) * local[0],
        uv_min[1] + (uv_max[1] - uv_min[1]) * local[1],
    ]
}

fn snap_px(value: f32) -> f32 {
    value.round()
}

const IRONEYE_ATLAS_SLOTS: usize = 7;
const IRONEYE_RETICLE_ATLAS_SLOTS: [usize; 3] = [0, 1, 2];
const IRONEYE_WEAKNESS_ATLAS_SLOTS: [usize; 4] = [3, 4, 5, 6];
const SCHOLAR_ENEMY_ATLAS_SLOTS: usize = 5;
const SCHOLAR_ENEMY_FULL_ATLAS_SLOT: usize = 0;
const SCHOLAR_ENEMY_POINTER_ATLAS_SLOT: usize = 1;
const SCHOLAR_ENEMY_POINTER_BACK_ATLAS_SLOT: usize = 2;
const SCHOLAR_ENEMY_WHITE_ATLAS_SLOT: usize = 3;
const SCHOLAR_ENEMY_RED_ATLAS_SLOT: usize = 4;
const REVENANT_SUMMON_TOP_UV_MAX_Y: f32 = 0.5;
const REVENANT_SUMMON_FRAME_UV_MIN: [f32; 2] = [0.5 / 1024.0, 0.5 / 512.0];
const REVENANT_SUMMON_FRAME_UV_MAX: [f32; 2] = [255.5 / 1024.0, 255.5 / 512.0];
const REVENANT_SUMMON_HP_UV_MIN: [f32; 2] = [0.0, 0.5];
const REVENANT_SUMMON_HP_UV_MAX: [f32; 2] = [408.0 / 1024.0, 280.0 / 512.0];
const REVENANT_SUMMON_DEBUG_VISIBLE: bool = false;
const REVENANT_GAME_SETTINGS_DEBUG_VISIBLE: bool = false;
const REVENANT_GAME_SETTINGS_DEBUG_BYTES: usize = 0x140;
static REVENANT_GAME_SETTINGS_BASELINE: LazyLock<Mutex<Option<[u8; REVENANT_GAME_SETTINGS_DEBUG_BYTES]>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Clone, Copy)]
enum ActiveTextureStyle {
    Default,
    UndertakerFreeUltimate,
}

fn self_scale(radius: f32) -> f32 {
    radius / 56.0
}

fn draw_png_icon(
    ui: &Ui,
    center: [f32; 2],
    radius: f32,
    icon_texture: Option<TextureRegion>,
    icon_texture_scale: f32,
    ready: bool,
) {
    if let Some(texture) = icon_texture {
        let draw = ui.get_foreground_draw_list();
        let half = radius * 1.03 * icon_texture_scale;
        let alpha = if ready { 255 } else { 178 };
        draw.add_image(
            texture.texture_id,
            [center[0] - half, center[1] - half],
            [center[0] + half, center[1] + half],
        )
        .uv_min(texture.uv_min)
        .uv_max(texture.uv_max)
        .col(ImColor32::from_rgba(255, 255, 255, alpha))
        .build();
    }
}

fn draw_ready_effect(
    ui: &Ui,
    center: [f32; 2],
    radius: f32,
    strength: f32,
    now: Instant,
    start: Instant,
) {
    let pulse = (((now - start).as_secs_f32() * 4.5).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
    let draw = ui.get_foreground_draw_list();
    let s = self_scale(radius);
    let outer_alpha = ((42.0 + 36.0 * pulse) * strength) as u8;
    let inner_alpha = ((90.0 + 45.0 * pulse) * strength) as u8;
    let highlight_alpha = ((150.0 + 50.0 * pulse) * strength) as u8;

    draw.add_circle(
        center,
        radius + 5.8 * s,
        ImColor32::from_rgba(92, 146, 255, outer_alpha),
    )
    .num_segments(96)
    .thickness(4.2 * s)
    .build();
    draw.add_circle(
        center,
        radius + 1.2 * s,
        ImColor32::from_rgba(210, 230, 255, inner_alpha),
    )
    .num_segments(96)
    .thickness(1.6 * s)
    .build();

    draw.add_circle(
        center,
        radius + 6.8 * s,
        ImColor32::from_rgba(178, 207, 255, highlight_alpha),
    )
    .num_segments(96)
    .thickness(2.2 * s)
    .build();
}

fn draw_ready_texture(
    ui: &Ui,
    center: [f32; 2],
    radius: f32,
    texture: TextureRegion,
    now: Instant,
    start: Instant,
) {
    let pulse = (((now - start).as_secs_f32() * 3.8).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
    let alpha = (210.0 + 45.0 * pulse) as u8;
    let draw = ui.get_foreground_draw_list();
    let half = radius * 1.31;
    draw.add_image(
        texture.texture_id,
        [center[0] - half, center[1] - half],
        [center[0] + half, center[1] + half],
    )
    .uv_min(texture.uv_min)
    .uv_max(texture.uv_max)
    .col(ImColor32::from_rgba(255, 255, 255, alpha))
    .build();
}

fn draw_active_texture(
    ui: &Ui,
    center: [f32; 2],
    radius: f32,
    texture: TextureRegion,
    layer: usize,
    style: ActiveTextureStyle,
    now: Instant,
    start: Instant,
) {
    let elapsed = (now - start).as_secs_f32();
    let speed = if layer == 0 { 3.8 } else { 5.1 };
    let phase = if layer == 0 { 0.0 } else { 1.35 };
    let pulse = ((elapsed * speed + phase).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
    if matches!(style, ActiveTextureStyle::Default) && layer == 1 {
        let draw = ui.get_foreground_draw_list();
        let cycle = (elapsed * 0.68).fract();
        let fade_in = (cycle / 0.18).clamp(0.0, 1.0);
        let fade_out = (1.0 - cycle).clamp(0.0, 1.0).powf(1.15);
        let scale = 1.16 + 0.24 * cycle;
        let alpha = (170.0 * fade_in * fade_out).clamp(0.0, 170.0) as u8;
        let half = radius * scale;
        draw.add_image(
            texture.texture_id,
            [center[0] - half, center[1] - half],
            [center[0] + half, center[1] + half],
        )
        .uv_min(texture.uv_min)
        .uv_max(texture.uv_max)
        .col(ImColor32::from_rgba(255, 255, 255, alpha))
        .build();
        return;
    }

    let (scale, alpha) = match style {
        ActiveTextureStyle::Default => (1.18, (190.0 + 45.0 * pulse) as u8),
        ActiveTextureStyle::UndertakerFreeUltimate => {
            if layer == 0 {
                (1.34, 245)
            } else {
                let cycle = (elapsed * 0.72 + if layer == 1 { 0.0 } else { 0.5 }).fract();
                let fade_in = (cycle / 0.18).clamp(0.0, 1.0);
                let fade_out = (1.0 - cycle).clamp(0.0, 1.0).powf(1.15);
                let scale = 1.31 + 0.30 * cycle;
                let alpha = (245.0 * fade_in * fade_out).clamp(0.0, 245.0) as u8;
                (scale, alpha)
            }
        }
    };
    let draw = ui.get_foreground_draw_list();
    let half = radius * scale;
    draw.add_image(
        texture.texture_id,
        [center[0] - half, center[1] - half],
        [center[0] + half, center[1] + half],
    )
    .uv_min(texture.uv_min)
    .uv_max(texture.uv_max)
    .col(ImColor32::from_rgba(255, 255, 255, alpha))
    .build();
}

fn draw_bottom_fill(ui: &Ui, center: [f32; 2], radius: f32, progress: f32, color: ImColor32) {
    if progress <= 0.0 {
        return;
    }

    let draw = ui.get_foreground_draw_list();
    let top = center[1] + radius - 2.0 * radius * progress;
    let mut y = top.max(center[1] - radius);
    while y <= center[1] + radius {
        let dy = y - center[1];
        let half_width = (radius * radius - dy * dy).max(0.0).sqrt();
        draw.add_line(
            [center[0] - half_width, y],
            [center[0] + half_width, y],
            color,
        )
        .thickness(2.0)
        .build();
        y += 2.0;
    }
}

fn draw_charge_count(ui: &Ui, center: [f32; 2], radius: f32, count: usize) {
    let s = self_scale(radius);
    let digit = count.min(2);
    let origin = [center[0] + radius * 0.57, center[1] + radius * 0.45];
    let text = digit.to_string();
    let font_size = (ui.current_font_size() * 2.48 * s).max(20.8 * s);
    let outline = 1.7 * s;
    let shadow = ImColor32::from_rgba(0, 0, 0, 235);

    for offset in [
        [-outline, 0.0],
        [outline, 0.0],
        [0.0, -outline],
        [0.0, outline],
        [outline, outline],
    ] {
        draw_sized_text(
            [origin[0] + offset[0], origin[1] + offset[1]],
            font_size,
            shadow,
            &text,
        );
    }
    draw_sized_text(
        origin,
        font_size,
        ImColor32::from_rgba(245, 249, 255, 255),
        &text,
    );
}

fn draw_sized_text(pos: [f32; 2], font_size: f32, color: ImColor32, text: &str) {
    unsafe {
        let draw = sys::igGetForegroundDrawList();
        let font = sys::igGetFont();
        if draw.is_null() || font.is_null() {
            return;
        }

        let text_begin = text.as_ptr() as *const c_char;
        let text_end = text_begin.add(text.len());
        let col: u32 = color.into();
        sys::ImDrawList_AddText_FontPtr(
            draw,
            font,
            font_size,
            sys::ImVec2 {
                x: pos[0],
                y: pos[1],
            },
            col,
            text_begin,
            text_end,
            0.0,
            std::ptr::null(),
        );
    }
}

fn draw_icon_mark(
    ui: &Ui,
    center: [f32; 2],
    radius: f32,
    label: &str,
    color: ImColor32,
    icon_scale: f32,
) {
    let draw = ui.get_foreground_draw_list();
    let s = self_scale(radius) * icon_scale;
    let white = ImColor32::from_rgba(235, 242, 255, 228);
    let pale = ImColor32::from_rgba(207, 219, 238, 180);

    if label == "SKILL" {
        draw.add_bezier_curve(
            [center[0] - 28.0 * s, center[1] + 16.0 * s],
            [center[0] - 12.0 * s, center[1] - 24.0 * s],
            [center[0] + 22.0 * s, center[1] - 24.0 * s],
            [center[0] + 12.0 * s, center[1] + 2.0 * s],
            white,
        )
        .thickness(3.8 * s)
        .num_segments(24)
        .build();
        draw.add_bezier_curve(
            [center[0] - 22.0 * s, center[1] + 12.0 * s],
            [center[0] - 5.0 * s, center[1] - 8.0 * s],
            [center[0] + 12.0 * s, center[1] - 18.0 * s],
            [center[0] + 28.0 * s, center[1] - 8.0 * s],
            color,
        )
        .thickness(2.6 * s)
        .num_segments(20)
        .build();
        draw.add_bezier_curve(
            [center[0] - 28.0 * s, center[1] - 5.0 * s],
            [center[0] - 8.0 * s, center[1] - 34.0 * s],
            [center[0] + 25.0 * s, center[1] - 27.0 * s],
            [center[0] + 30.0 * s, center[1] - 3.0 * s],
            pale,
        )
        .thickness(1.5 * s)
        .num_segments(24)
        .build();
        draw.add_circle([center[0] + 22.0 * s, center[1] - 17.0 * s], 6.0 * s, white)
            .num_segments(32)
            .thickness(1.8 * s)
            .build();
    } else {
        draw.add_line(
            [center[0] - 21.0 * s, center[1] + 22.0 * s],
            [center[0] + 25.0 * s, center[1] - 24.0 * s],
            white,
        )
        .thickness(5.0 * s)
        .build();
        draw.add_line(
            [center[0] - 29.0 * s, center[1] - 1.0 * s],
            [center[0] + 3.0 * s, center[1] + 31.0 * s],
            color,
        )
        .thickness(3.2 * s)
        .build();
        draw.add_line(
            [center[0] - 30.0 * s, center[1] + 22.0 * s],
            [center[0] + 33.0 * s, center[1] - 41.0 * s],
            pale,
        )
        .thickness(1.6 * s)
        .build();
        draw.add_triangle(
            [center[0] - 27.0 * s, center[1] + 23.0 * s],
            [center[0] - 16.0 * s, center[1] + 11.0 * s],
            [center[0] - 9.0 * s, center[1] + 28.0 * s],
            white,
        )
        .filled(true)
        .build();
        draw.add_bezier_curve(
            [center[0] - 42.0 * s, center[1] - 31.0 * s],
            [center[0] - 12.0 * s, center[1] - 48.0 * s],
            [center[0] + 18.0 * s, center[1] - 44.0 * s],
            [center[0] + 43.0 * s, center[1] - 29.0 * s],
            pale,
        )
        .thickness(1.4 * s)
        .num_segments(24)
        .build();
        draw.add_bezier_curve(
            [center[0] - 41.0 * s, center[1] - 19.0 * s],
            [center[0] - 9.0 * s, center[1] - 35.0 * s],
            [center[0] + 16.0 * s, center[1] - 33.0 * s],
            [center[0] + 41.0 * s, center[1] - 17.0 * s],
            pale,
        )
        .thickness(1.2 * s)
        .num_segments(24)
        .build();
    }
}
