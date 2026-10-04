--!strict
-- GroundMan's own chip, RC Brakr (the original's `sub_80F1C1C`, his action
-- table's entry 0xA): he readies his drill (13 ticks), then digs along his
-- row at 4 pixels a tick, invulnerable, his drill on him
-- (chips/grndman/drill), hitting each panel he enters. Reaching the far
-- column he brings down nine rocks (chips/grndman/rock), one every 20
-- ticks, each on a panel with an enemy navi, the camera shaking; stopped on
-- the way (no solid ground, a body of the other side's or an object), he
-- drops none. Then he comes up and is back on his panel. See
-- docs/engine/standard-chips.md, "Action 0x0A".

local collision = require("../../rules/collision")
local drill = require("../../chips/grndman/drill")
local link = require("../../lib/link_chips")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local rock = require("../../chips/grndman/rock")
local sparks = require("../../lib/sparks")

export type State = {
    timer: number,
    -- The rocks left to drop (the attack's parameter byte, +0x0C).
    rocks: number,
    -- His drill (AIAttackVars+0x30).
    drill: Object?,
}

local STATE = { timer = "u16", rocks = "u8", drill = "object" }

local READY, DIG, ROCKFALL, RISE, HOME, REST = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14
local ANIM_READY, ANIM_DIG, ANIM_RISE, ANIM_HOME = 0x11, 0x12, 4, 3
local SOUND_DRILL, SOUND_ROCKFALL = asset.sound("bn6:drill-spin"), asset.sound("bn6:rockfall")

local READY_TICKS, RISE_TICKS, HOME_TICKS, REST_TICKS = 0xC, 4, 9, 0x14

-- The dig: 4 pixels a tick to the far column of his side's forward (a
-- panel is 40 pixels wide), invulnerable for as long as it takes.
local SPEED, PANEL_WIDTH = 0x4_0000, 0x28_0000
local FAR_COLUMN = { [0] = 6, 1 }
local INVULNERABLE_TICKS = 0xFFFF

-- A panel he digs on through, by side (`byte_80F1D20`): solid ground, with
-- no enemy navi's body and no object of either side's or neutral.
local DIGGABLE = { [0] = { 0x10, 0x0780_0000 }, { 0x10, 0x0B80_0000 } }

-- The hit on each panel he enters: a breaking attack at navis, with the
-- breaking spark.
local DIG_HIT_MOD = 0x63

-- The rockfall: nine rocks, 20 ticks apart, the camera shaking for each.
local ROCKS, ROCK_TICKS = 9, 0x14
local SHAKE, SHAKE_TICKS = 1, 0x14

-- Where a rock falls, by side (`off_80F1EF0`): a panel of the other side's
-- area with an enemy navi's body.
local ROCK_TARGETS = { [0] = { 0x0400_0020, 0 }, { 0x0800_0000, 0x20 } }

-- His rocks: 10 ticks' warning, waiting a dimming out, not cracking the
-- panel.
local ROCK: GroundRockDrop = { warning = 0xA, while_dimmed = false, cracks = false, silent = false }

-- `sub_80F1C48`: the drill readied, nine rocks to drop; 12 ticks after the
-- entry.
local function ready(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        s.rocks = ROCKS
        me.anim = ANIM_READY
        s.timer = READY_TICKS
    elseif link.run_out(s) then
        link.go(me, DIG)
    end
end

-- `sub_80F1E50`: off toward the far column; the ticks it takes (a BIOS
-- division).
local function set_off(me: Object, s: State)
    me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
    s.timer = int.tdiv(math.abs(FAR_COLUMN[me.alliance] - me.panel_x) * PANEL_WIDTH, SPEED)
end

-- `sub_80F1E7E`: the hit on the panel he enters.
local function dig_hit(me: Object, x: number, y: number)
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.attack_element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.breaking,
        target = collision.hits_navis,
        self_type = collision.breaking,
        damage = link.damage(me),
        stamina = link.stamina(me),
        hit_mod = DIG_HIT_MOD,
    })
end

-- `sub_80F1C78`: the dig. Its entry sets him off, invulnerable, with his
-- drill; each tick after he moves on, hits a column he enters, and stops
-- (coming up at once) on a panel he can't dig through; at the far column,
-- the rockfall.
local function dig(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_DIG
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me:set_status("moving", true)
        me:set_status("using_action", true)
        me:set_status_timer("invulnerable", INVULNERABLE_TICKS)
        me:set_status("invulnerable", true)
        set_off(me, s)
        battle.play_sound(SOUND_DRILL)
        s.drill = drill.put_on_for_action(me)
        return
    end
    local column = me.panel_x
    local p = me.pos
    local x = int.i32(p.x + me.vel.x)
    me.pos = Vec3.new(x, p.y, p.z)
    local px, py = link.panel_at(x, p.y)
    if px ~= column then
        dig_hit(me, px, py)
    end
    local ok = DIGGABLE[me.alliance]
    if not field.check(px, py, ok[1], ok[2]) then
        link.go(me, RISE)
        return
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if link.run_out(s) then
        battle.play_sound(SOUND_ROCKFALL)
        link.go(me, ROCKFALL)
    end
end

-- `sub_80F1E98`: a rock on one of the panels with an enemy navi (his own
-- panel left out), picked with one draw; none without such a panel (and
-- no draw).
local function drop_rock(me: Object)
    local want = ROCK_TARGETS[me.alliance]
    local found = panels.filtered_except(me, want[1], want[2])
    if #found == 0 then
        return
    end
    local p = found[int.tmod(battle.rng_positive(), #found) + 1]
    rock.drop(me, panels.x(p), panels.y(p), me.attack_element, link.damage_word(me), ROCK, 0)
end

-- `sub_80F1D30`: the rockfall: a rock and a camera shake at each entry;
-- 20 ticks, then the entry again while rocks are left.
local function rockfall(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        drop_rock(me)
        battle.shake_camera(SHAKE, SHAKE_TICKS)
        s.timer = ROCK_TICKS
    elseif link.run_out(s) then
        local left = s.rocks - 1
        s.rocks = left
        if left > 0 then
            me.step_init = 0
        else
            link.go(me, RISE)
        end
    end
end

-- `sub_80F1D6C`: coming up, the drill off; 4 ticks.
local function rise(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_RISE
        s.timer = RISE_TICKS
        drill.take_off(s.drill)
    elseif link.run_out(s) then
        link.go(me, HOME)
    end
end

-- `sub_80F1D9A`: back on his panel, the move over, vulnerable again
-- (`sub_800EB08`); 9 ticks.
local function home(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:unreserve_panel(me.panel_x, me.panel_y)
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me:set_status("moving", false)
        me:set_status("using_action", false)
        me:set_status_timer("invulnerable", 0)
        me:set_status("invulnerable", false)
        me.anim = ANIM_HOME
        s.timer = HOME_TICKS
    elseif link.run_out(s) then
        link.go(me, REST)
    end
end

-- `sub_80F1DE4`: 20 ticks, then idle.
local function rest(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0
        s.timer = REST_TICKS
    elseif link.run_out(s) then
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [READY] = ready, [DIG] = dig, [ROCKFALL] = rockfall, [RISE] = rise, [HOME] = home, [REST] = rest }

return define.chip {
    id = "bn6:rc-brakr",
    codes = { "G" },
    element = "null",
    family = "break",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = { formula = "navi_level", base = 30, per_level = 10 },
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    library = { number = 408, index = 0, sort = 7 },
    slot_in_limit = 1,
    -- `sub_80F1C1C` (`off_80F1C30`).
    action = define.action {
        id = "bn6:rc-brakr/action",
        state = STATE,
        update = function(me: Object, s: State)
            link.run(me, s, PHASES, "RC Brakr (sub_80F1C1C)")
        end,
    },
}
