--!strict
-- One-shot effects many kinds show (`battle.effect(pos, effects.explosion)`):
-- a sprite's animation that plays once. Effects one owner shows live with
-- it.

local effects = {}

-- An explosion: a blast landing, something breaking (effect 0x00).
effects.explosion = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }

-- A splash of bubbles: something landing in a hole (effect 0x01).
effects.splash = define.effect { sprite = asset.sprite("rising-bubble"), anim = 0, palette = 0 }

-- A puff of smoke: a thrown thing gone when the battle ends, a field
-- object a chip removes (effect 0x14).
effects.puff = define.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }

-- A burst: a meteor or a fist landing, a snake striking (effect 0x05).
effects.burst = define.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0 }

-- A flash: a gun's shot landing, MegaMan's volley in DblHero (effect
-- 0x21).
effects.flash = define.effect { sprite = asset.sprite("reflected-shot"), anim = 0, palette = 0 }

-- The rest of the original's effects (gen-content).
-- Effect 0x02.
effects.rising_bubble_1 = define.effect { sprite = asset.sprite("rising-bubble"), anim = 1, palette = 0 }
-- Effect 0x03.
effects.deletion = define.effect { sprite = asset.sprite("deletion"), anim = 0, palette = 0 }
-- Effect 0x04.
effects.ground_drill_effect = define.effect { sprite = asset.sprite("ground-drill-effect"), anim = 0, palette = 0 }
-- Effect 0x06.
effects.heal = define.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0 }
-- Effect 0x07.
effects.copy_mark = define.effect { sprite = asset.sprite("copy-mark"), anim = 0, palette = 0 }
-- Effect 0x08.
effects.copy_mark_2 = define.effect { sprite = asset.sprite("copy-mark"), anim = 2, palette = 0 }
-- Effect 0x09.
effects.copy_mark_3 = define.effect { sprite = asset.sprite("copy-mark"), anim = 3, palette = 0 }
-- Effect 0x0A.
effects.copy_mark_4 = define.effect { sprite = asset.sprite("copy-mark"), anim = 4, palette = 0 }
-- Effect 0x0B.
effects.copy_mark_5 = define.effect { sprite = asset.sprite("copy-mark"), anim = 5, palette = 0 }
-- Effect 0x0C.
effects.copy_mark_1 = define.effect { sprite = asset.sprite("copy-mark"), anim = 1, palette = 0 }
-- Effect 0x0D.
effects.copy_mark_9 = define.effect { sprite = asset.sprite("copy-mark"), anim = 9, palette = 0 }
-- Effect 0x0E.
effects.effect_0e = define.effect { sprite = asset.sprite("effect-0e"), anim = 0, palette = 0 }
-- Effect 0x0F.
effects.hit = define.effect { sprite = asset.sprite("hit"), anim = 0, palette = 0 }
-- Effect 0x10.
effects.reflected_shot_1 = define.effect { sprite = asset.sprite("reflected-shot"), anim = 1, palette = 0 }
-- Effect 0x12.
effects.puff_2 = define.effect { sprite = asset.sprite("puff"), anim = 0, palette = 0 }
-- Effect 0x13.
effects.puff_1 = define.effect { sprite = asset.sprite("puff"), anim = 1, palette = 0 }
-- Effect 0x15.
effects.puff_3 = define.effect { sprite = asset.sprite("puff"), anim = 3, palette = 0 }
-- Effect 0x16.
effects.sword_slash = define.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }
-- Effect 0x17.
effects.sword_slash_1 = define.effect { sprite = asset.sprite("sword-slash"), anim = 1, palette = 0 }
-- Effect 0x18.
effects.sword_slash_2 = define.effect { sprite = asset.sprite("sword-slash"), anim = 2, palette = 0 }
-- Effect 0x19.
effects.sword_slash_19 = define.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 5 }
-- Effect 0x1A.
effects.sword_slash_1_1a = define.effect { sprite = asset.sprite("sword-slash"), anim = 1, palette = 5 }
-- Effect 0x1B.
effects.big_slash = define.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 0 }
-- Effect 0x1C.
effects.big_slash_1c = define.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 2 }
-- Effect 0x1E.
effects.effect_1e = define.effect { sprite = asset.sprite("effect-1e"), anim = 0, palette = 0 }
-- Effect 0x1F.
effects.spout_splash = define.effect { sprite = asset.sprite("spout-splash"), anim = 0, palette = 0 }
-- Effect 0x20.
effects.effect_20 = define.effect { sprite = asset.sprite("effect-20"), anim = 0, palette = 0 }
-- Effect 0x22.
effects.effect_22 = define.effect { sprite = asset.sprite("effect-22"), anim = 0, palette = 0 }
-- Effect 0x23.
effects.energy_burst = define.effect { sprite = asset.sprite("energy-burst"), anim = 0, palette = 1 }
-- Effect 0x24.
effects.explosion_24 = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 1 }
-- Effect 0x25.
effects.sword_slash_3 = define.effect { sprite = asset.sprite("sword-slash"), anim = 3, palette = 0 }
-- Effect 0x26.
effects.elmnt_ice = define.effect { sprite = asset.sprite("elmnt-ice"), anim = 0, palette = 0 }
-- Effect 0x28.
effects.copy_mark_11 = define.effect { sprite = asset.sprite("copy-mark"), anim = 11, palette = 1 }
-- Effect 0x29.
effects.blast_fire_12 = define.effect { sprite = asset.sprite("blast-fire"), anim = 12, palette = 0 }
-- Effect 0x2A.
effects.spout_man_effect = define.effect { sprite = asset.sprite("spout-man-effect"), anim = 0, palette = 0 }
-- Effect 0x2B.
effects.effect_2b_21 = define.effect { sprite = asset.sprite("effect-2b"), anim = 21, palette = 0 }
-- Effect 0x2C.
effects.effect_2b_25 = define.effect { sprite = asset.sprite("effect-2b"), anim = 25, palette = 0 }
-- Effect 0x2D.
effects.sword_slash_1_2d = define.effect { sprite = asset.sprite("sword-slash"), anim = 1, palette = 6 }
-- Effect 0x2E.
effects.form_change = define.effect { sprite = asset.sprite("form-change"), anim = 0, palette = 0 }
-- Effect 0x2F.
effects.effect_2f = define.effect { sprite = asset.sprite("effect-2f"), anim = 0, palette = 0 }
-- Effect 0x30.
effects.dust = define.effect { sprite = asset.sprite("dust"), anim = 0, palette = 0 }
-- Effect 0x31.
effects.buster_up = define.effect { sprite = asset.sprite("buster-up"), anim = 0, palette = 0 }
-- Effect 0x32.
effects.effect_32_10 = define.effect { sprite = asset.sprite("effect-32"), anim = 10, palette = 0 }
-- Effect 0x33.
effects.big_slash_1 = define.effect { sprite = asset.sprite("big-slash"), anim = 1, palette = 0 }
-- Effect 0x34.
effects.dust_2 = define.effect { sprite = asset.sprite("dust-2"), anim = 0, palette = 0 }
-- Effect 0x35.
effects.colonel_effect = define.effect { sprite = asset.sprite("colonel-effect"), anim = 0, palette = 0 }
-- Effect 0x36.
effects.colonel_effect_1 = define.effect { sprite = asset.sprite("colonel-effect"), anim = 1, palette = 0 }
-- Effect 0x37.
effects.colonel_effect_2 = define.effect { sprite = asset.sprite("colonel-effect"), anim = 2, palette = 0 }
-- Effect 0x38.
effects.colonel_effect_3 = define.effect { sprite = asset.sprite("colonel-effect"), anim = 3, palette = 0 }
-- Effect 0x39.
effects.slash_man_effect = define.effect { sprite = asset.sprite("slash-man-effect"), anim = 0, palette = 0 }
-- Effect 0x3A.
effects.slash_man_effect_1 = define.effect { sprite = asset.sprite("slash-man-effect"), anim = 1, palette = 0 }
-- Effect 0x3B.
effects.grab_shot_1 = define.effect { sprite = asset.sprite("grab-shot"), anim = 1, palette = 0 }
-- Effect 0x3C.
effects.grab_shot_1_3c = define.effect { sprite = asset.sprite("grab-shot"), anim = 1, palette = 8 }
-- Effect 0x3D.
effects.reflected_shot_3d = define.effect { sprite = asset.sprite("reflected-shot"), anim = 0, palette = 2 }
-- Effect 0x3E.
effects.beast_over_blast = define.effect { sprite = asset.sprite("form-change"), anim = 1, palette = 0 }
-- Effect 0x3F.
effects.beast_over_gregar = define.effect { sprite = asset.sprite("form-change"), anim = 2, palette = 0 }
-- Effect 0x40.
effects.beast_over_falzar = define.effect { sprite = asset.sprite("form-change"), anim = 3, palette = 0 }
-- Effect 0x41.
effects.swirl_1 = define.effect { sprite = asset.sprite("swirl"), anim = 1, palette = 1 }
-- Effect 0x43.
effects.moon_blade = define.effect { sprite = asset.sprite("moon-blade"), anim = 0, palette = 0 }
-- Effect 0x44.
effects.swirl = define.effect { sprite = asset.sprite("swirl"), anim = 0, palette = 0 }
-- Effect 0x45.
effects.beast_over_burst_45 = define.effect { sprite = asset.sprite("beast-over-burst"), anim = 0, palette = 0 }
-- Effect 0x46.
effects.copy_mark_7 = define.effect { sprite = asset.sprite("copy-mark"), anim = 7, palette = 0 }
-- Effect 0x47.
effects.blast = define.effect { sprite = asset.sprite("blast"), anim = 0, palette = 0 }
-- Effect 0x48.
effects.copy_mark_12 = define.effect { sprite = asset.sprite("copy-mark"), anim = 12, palette = 0 }
-- Effect 0x49.
effects.sight = define.effect { sprite = asset.sprite("sight"), anim = 0, palette = 0 }
-- Effect 0x4A.
effects.explosion_4a = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 2 }
-- Effect 0x4B.
effects.effect_4b_82 = define.effect { sprite = asset.sprite("effect-4b"), anim = 82, palette = 16 }
-- Effect 0x4C.
effects.slash_man_effect_2 = define.effect { sprite = asset.sprite("slash-man-effect"), anim = 2, palette = 0 }
-- Effect 0x4D.
effects.slash_man_effect_3 = define.effect { sprite = asset.sprite("slash-man-effect"), anim = 3, palette = 0 }
-- Effect 0x4E.
effects.charge_glow = define.effect { sprite = asset.sprite("charge-glow"), anim = 0, palette = 0 }
-- Effect 0x4F.
effects.charge_glow_2 = define.effect { sprite = asset.sprite("charge-glow"), anim = 2, palette = 0 }
-- Effect 0x50.
effects.sand_hole = define.effect { sprite = asset.sprite("sand-hole"), anim = 0, palette = 0 }
-- Effect 0x51.
effects.dustman_24 = define.effect { sprite = asset.sprite("dustman"), anim = 24, palette = 0 }
-- Effect 0x52.
effects.cross_slash = define.effect { sprite = asset.sprite("cross-slash"), anim = 0, palette = 0 }
-- Effect 0x53.
effects.copy_mark_1_53 = define.effect { sprite = asset.sprite("copy-mark"), anim = 1, palette = 6 }
-- Effect 0x54.
effects.panel_strike = define.effect { sprite = asset.sprite("panel-strike"), anim = 0, palette = 0 }
-- Effect 0x55.
effects.effect_55_12 = define.effect { sprite = asset.sprite("effect-55"), anim = 12, palette = 0 }
-- Effect 0x56.
effects.sight_4 = define.effect { sprite = asset.sprite("sight"), anim = 4, palette = 0 }
-- Effect 0x57.
effects.copy_mark_8 = define.effect { sprite = asset.sprite("copy-mark"), anim = 8, palette = 0 }
-- Effect 0x58.
effects.effect_2f_1 = define.effect { sprite = asset.sprite("effect-2f"), anim = 1, palette = 0 }
-- Effect 0x59.
effects.tornado = define.effect { sprite = asset.sprite("tornado"), anim = 0, palette = 0 }
-- Effect 0x5A.
effects.hub = define.effect { sprite = asset.sprite("hub"), anim = 0, palette = 0 }
-- Effect 0x5B.
effects.bug = define.effect { sprite = asset.sprite("bug"), anim = 0, palette = 0 }
-- Effect 0x5C.
effects.charge_glow_a_3 = define.effect { sprite = asset.sprite("charge-glow-a"), anim = 3, palette = 1 }
-- Effect 0x5D.
effects.effect_5d = define.effect { sprite = asset.sprite("effect-5d"), anim = 0, palette = 0 }
-- Effect 0x5E.
effects.dustman_30 = define.effect { sprite = asset.sprite("dustman"), anim = 30, palette = 0 }
-- Effect 0x60.
effects.effect_60 = define.effect { sprite = asset.sprite("effect-60"), anim = 0, palette = 0 }
-- Effect 0x61.
effects.effect_61 = define.effect { sprite = asset.sprite("effect-61"), anim = 0, palette = 0 }
-- Effect 0x62.
effects.colonel_effect_5 = define.effect { sprite = asset.sprite("colonel-effect"), anim = 5, palette = 0 }
-- Effect 0x63.
effects.dust_cloud = define.effect { sprite = asset.sprite("dust-cloud"), anim = 0, palette = 0 }
-- Effect 0x64.
effects.sword_slash_64 = define.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 6 }
-- Effect 0x65.
effects.sword_slash_1_65 = define.effect { sprite = asset.sprite("sword-slash"), anim = 1, palette = 4 }
-- Effect 0x66.
effects.sword_slash_66 = define.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 4 }
-- Effect 0x67.
effects.sword_slash_3_67 = define.effect { sprite = asset.sprite("sword-slash"), anim = 3, palette = 4 }
-- Effect 0x69.
effects.hub_1 = define.effect { sprite = asset.sprite("hub"), anim = 1, palette = 0 }
-- Effect 0x6A.
effects.effect_20_6a = define.effect { sprite = asset.sprite("effect-20"), anim = 0, palette = 1 }

return effects
