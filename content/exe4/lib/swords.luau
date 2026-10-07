--!strict
-- EXE4's swords: the chips' action 0x0A (0x080EB51E), a navi holding a blade and slashing the panels ahead, one hit
-- on the swing's 10th tick and an effect that draws the slash. EXE6's slash (@exelib/swords/slash) in its phases
-- and timing, but for EXE4's blade (its own attachment, row 3) and the recovery, which leaves the navi's moving
-- flag. Each chip composes its own from its variant's rows (`swords.action { ... }`): the hit (0x080EB7B0's region
-- word: the region, the target and self collision rows), its hit modifier (0x080EB7F1), the slash's effect
-- (0x080EB7E4, a row of the effect table 0x080E1698), the swing's sound (0x8E; variants 5 and 11 0xB4), the swings
-- (variant 11's two, 0x080EB5E4) and a second hit on the same panel (variant 10's: 0x0705FF01).
--
-- A step sword (a chip whose parameter +0x10 is set) first steps two panels ahead, if that panel is on the field
-- with nobody on it, leaving an afterimage, then two more as it swings, and steps back after the swing; with nowhere
-- to step it doesn't swing. (ProtoSoul, soul 7, swings blade 13 and takes its hit modifiers from 0x080EB7FE: the
-- souls' to wire, docs/design/exe4-map.md §18 item 70.)

local attachment = require("../objects/attachment")

local swords = {}

-- The blade a navi holds (attachment row 3, 0x080CD524: 0c-00 in palette 0).
swords.blade = attachment.look { sprite = asset.sprite("sword"), palette = 0 }

-- The swings' sounds (0x080EB612).
swords.swing = asset.sound("sword-swing")
swords.big_swing = asset.sound("big-sword-swing")

local SLASH, BIG_SLASH = asset.sprite("sword-slash"), asset.sprite("big-slash")

-- The slashes' effects by the variants' rows of the effect table (0x080EB7E4: 0x080E1698's rows).
swords.effects = {
    -- 0x17 to 0x19: over a column, two panels, one panel.
    wide = new.effect { sprite = SLASH, anim = 0, palette = 0 },
    long = new.effect { sprite = SLASH, anim = 1, palette = 0 },
    slash = new.effect { sprite = SLASH, anim = 2, palette = 0 },
    -- 0x1A and 0x1B: the wide and long ones in palette 5.
    wide_blade = new.effect { sprite = SLASH, anim = 0, palette = 5 },
    long_blade = new.effect { sprite = SLASH, anim = 1, palette = 5 },
    -- 0x1D: a big slash over two columns; 0x1E: the dark one.
    big = new.effect { sprite = BIG_SLASH, anim = 0, palette = 0 },
    dark = new.effect { sprite = BIG_SLASH, anim = 0, palette = 2 },
}

-- A sword's hit (a variant's region word, 0x080EB7B0: its region, the collision rows it hits and is) and modifier.
export type SwordHit = {
    region: Region,
    hits: CollisionType,
    is: CollisionType,
    hit_mod: number,
}

export type SwordSpec = {
    id: string,
    hit: SwordHit,
    -- Variant 10's second hit on the panel (0x080EB728: region 1, the same rows and modifier).
    second_region: Region?,
    effect: Effect,
    sound: Sound,
    -- Variant 11's two swings (0x080EB5E4); one else.
    swings: number?,
    -- A step sword (the chip's parameter +0x10).
    steps: boolean?,
}

type State = {
    -- Ticks left in the phase (the attack's +0x10). (The swings left are the navi's `attack_count`.)
    timer: number,
}

-- The phases, on the attack's step (0x080EB534's table).
local START, SET_UP, SWING, STEP_BACK, RECOVER = 0x0, 0x4, 0x8, 0xC, 0x10

-- The panel a step sword steps to: on the field (every panel type's flag 0x10000), with no body on it.
local PANEL_EXISTS, BODIES = 0x10000, 0x0F800000
-- The afterimages' color shader.
local AFTERIMAGE_SHADER = 0xFFE0
-- The blade's animation: the navi's number (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- `object_getFlip`.
local function flip(me: Object): number
    return bit32.bxor(me.alliance, me.flip)
end

-- The damage word: the chip's damage and hit parameter, plus the attack's +6 halfword.
local function damage_word(me: Object): number
    return int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
end

-- The chips' action 0x0A with a variant's rows.
function swords.action(spec: SwordSpec): Action
    -- 0x0800AD2A: not while moving or sliding; then moving, from its panel, which it holds.
    local function set_off(me: Object): boolean
        if me:status("moving") or me:status("sliding") then
            return false
        end
        me:set_status("moving", true)
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        return true
    end

    -- 0x080EB548: on to the swing; a step sword steps two panels ahead first.
    local function start(me: Object, _s: State)
        set_step(me, SET_UP)
        if not spec.steps or not set_off(me) then
            return
        end
        local x, y = me.panel_x + 2 * me.facing, me.panel_y
        if not field.check(x, y, PANEL_EXISTS, BODIES) then
            me:set_animation(4)
            set_step(me, STEP_BACK)
            return
        end
        battle.afterimage(me, me.pos, {
            anim = 0,
            flip = flip(me),
            color_shader = AFTERIMAGE_SHADER,
            lifetime = 0x14,
            shadow = "ground",
        })
        me.panel_x, me.panel_y = x, y
        me:set_coordinates_from_panel()
        me:update_collision_panels()
    end

    -- 0x080EB5E4: one swing, or variant 11's two.
    local function set_up(me: Object, _s: State)
        me.attack_count = spec.swings or 1
        me.marker = 0
        set_step(me, SWING)
    end

    -- A hit on the panel ahead.
    local function hit(me: Object, x: number, y: number, region: Region, word: number)
        local h = spec.hit
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.attack_element,
            region = region,
            -- None (the word's 0xFF).
            hit_effect = nil,
            target = h.hits,
            self_type = h.is,
            damage = bit32.band(word, 0xFFFF),
            stamina = bit32.rshift(word, 16),
            hit_mod = h.hit_mod,
        })
    end

    -- 0x080EB6DE, on the swing's 10th tick: the hit region on the panel ahead (variant 10's second), and the
    -- slash's effect over it.
    local function strike(me: Object)
        local x, y = me.panel_x + me.facing, me.panel_y
        local word = damage_word(me)
        hit(me, x, y, spec.hit.region, word)
        if spec.second_region then
            hit(me, x, y, spec.second_region, word)
        end
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, 0x10 * 0x10000), spec.effect, flip(me))
    end

    -- 0x080EB5FA: hold the blade and swing: a step sword leaves two more afterimages (itself and its blade) on the
    -- 6th tick, the slash lands on the 10th, and the swing lasts 22.
    local function swing(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(5)
            me:open_counter_window()
            battle.play_sound(spec.sound)
            attachment.attach(me, swords.blade, "related", NAVI_NUMBER)
            me:set_status("using_action", true)
            s.timer = 0x15
            me.step_init = 4
        end
        if s.timer == 0x10 and spec.steps then
            battle.afterimage(me, me.pos, {
                anim = 5,
                flip = flip(me),
                color_shader = AFTERIMAGE_SHADER,
                lifetime = 0x1E,
                shadow = "ground",
            })
            battle.afterimage(me, me.pos, {
                sprite = swords.blade.sprite,
                anim = NAVI_NUMBER,
                flip = flip(me),
                color_shader = AFTERIMAGE_SHADER,
                lifetime = 0x1E,
                shadow = "with_sprite",
            })
        end
        if s.timer == 0x0C then
            strike(me)
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        local swings = me.attack_count - 1
        me.attack_count = swings
        if swings > 0 then
            -- The next swing: a fresh blade.
            me.related1 = nil
            me.step_init = 0
            return
        end
        set_step(me, if spec.steps then STEP_BACK else RECOVER)
    end

    -- 0x080EB80C: a step sword goes back where it stepped from once its animation ends.
    local function step_back(me: Object, _s: State)
        if not bit32.btest(me.sprite.frame_flags, 0x80) then
            return
        end
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:unreserve_panel(me.panel_x, me.panel_y)
        me:update_collision_panels()
        me:set_coordinates_from_panel()
        set_step(me, RECOVER)
    end

    -- 0x080EB832: 6 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = 5
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        me.related1 = nil
        me:exit_attack()
    end

    -- 0x080EB51E.
    local function update(me: Object, s: State)
        local step = me.step
        if step == START then
            start(me, s)
        elseif step == SET_UP then
            set_up(me, s)
        elseif step == SWING then
            swing(me, s)
        elseif step == STEP_BACK then
            step_back(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("sword phase %#x reads past its phase table", step))
        end
    end

    return new.action { id = spec.id, state = { timer = "i16" }, update = update }
end

return swords
