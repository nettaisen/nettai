--!strict
-- SlashMan's own chip, RSlash (the original's `sub_80F0CB0`, his action
-- table's entry 0xA): he curls up (21 ticks), then rolls around the field
-- at 4 pixels a tick with a riding hit on him (navis/slashman/riding_hit):
-- from the bottom row straight at the enemy, else down to the bottom row
-- first; along the bottom row until an enemy navi is in his column or he
-- reaches the far column; up to the top row; back along it to his column;
-- and down to his panel. A panel that isn't solid ground, or holds a
-- neutral object, stops the roll. Then he lands back on his panel. See
-- docs/engine/standard-chips.md, "Action 0x0A".

local collision = require("../../rules/collision")
local link = require("../../lib/link_chips")
local panels = require("@common/panels")
local regions = require("@common/regions")
local riding_hit = require("./riding_hit")

export type State = {
    timer: number,
    -- The riding hit (AIAttackVars+0x30).
    hit: Object?,
    -- His speed, 16.16 (+0x34).
    speed: number,
    -- The leg of his route he is on (the attack's parameter word, +0x0C,
    -- whose low byte the route reads; a turn adds 1 to the whole word).
    leg: number,
}

local STATE = { timer = "u16", hit = "object", speed = "i32", leg = "u32" }

local CURL, LAUNCH, ROLL, LAND = 0x0, 0x4, 0x8, 0xC
local ANIM_CURL, ANIM_ROLL, ANIM_LAND = 0x12, 0x13, 7
local SPEED = 0x4_0000
local SOUND = asset.sound("bn6:rslash")

local CURL_TICKS, LAUNCH_TICKS, LAND_TICKS = 0x14, 0x0A, 0x14

-- The riding hit: a slash at navis, hit modifier 3, no spark; it goes on
-- after hitting a body.
local HIT: RidingHit = { is = collision.slash, hits = collision.hits_navis, hit_mod = 3, spark = nil, keeps_going = true }

-- What he rolls onto (`dword_80F0DE4`): on the field and solid, with no
-- neutral object.
local ROLL_REQUIRE, ROLL_FORBID = 0x0001_0010, 0x0080_0000

-- His row along the bottom.
local BOTTOM_ROW = 3

-- The route, by side and leg (`byte_80F0F50`): the column or row where
-- the leg turns (0: none), and the next leg's direction. Legs 3 and 4 turn
-- and end at his own panel instead.
type Leg = { x: number, y: number, dx: number, dy: number }
local ROUTE: { [number]: { [number]: Leg } } = {
    [0] = {
        [0] = { x = 0, y = 3, dx = 1, dy = 0 },
        { x = 6, y = 0, dx = 0, dy = -1 },
        { x = 0, y = 1, dx = -1, dy = 0 },
        { x = 1, y = 0, dx = 0, dy = 1 },
        { x = 0, y = 0, dx = 0, dy = 0 },
    },
    [1] = {
        [0] = { x = 0, y = 3, dx = -1, dy = 0 },
        { x = 1, y = 0, dx = 0, dy = -1 },
        { x = 0, y = 1, dx = 1, dy = 0 },
        { x = 6, y = 0, dx = 0, dy = 1 },
        { x = 0, y = 0, dx = 0, dy = 0 },
    },
}

-- An enemy navi's body, by side (`off_80F0F98`).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }

-- The leg the route reads: the parameter word's low byte.
local function leg(s: State): number
    return bit32.band(s.leg, 0xFF)
end

-- `sub_80F0F78`: an enemy navi in his column.
local function enemy_in_column(me: Object): boolean
    return #panels.in_column(me.panel_x, ENEMY_NAVI[me.alliance], 0) ~= 0
end

-- `sub_80F0EC0`, at a panel's center: turn onto the next leg where this
-- one turns; true once he is back on his own panel.
local function route(me: Object, s: State): boolean
    local n = leg(s)
    local r = ROUTE[me.alliance][n]
    if r == nil then
        error(string.format("RSlash leg %d reads past byte_80F0F50", n))
    end
    local function turn(): boolean
        me.vel = Vec3.new(r.dx * s.speed, r.dy * s.speed, me.vel.z)
        s.leg = s.leg + 1
        return false
    end
    -- `loc_80F0F16`: in his column, home on his row, else down to it.
    local function at_home_column(): boolean
        if me.future_panel_x ~= me.panel_x then
            return false
        end
        if me.future_panel_y ~= me.panel_y then
            return turn()
        end
        return true
    end
    if n == 4 then
        -- `loc_80F0F2A`: down his column, home on his row.
        return me.future_panel_y == me.panel_y
    elseif n == 3 then
        return at_home_column()
    elseif n == 1 and enemy_in_column(me) then
        return turn()
    end
    if r.x ~= 0 then
        return if me.panel_x == r.x then turn() else false
    elseif r.y ~= 0 then
        return if me.panel_y == r.y then turn() else false
    end
    return at_home_column()
end

-- `sub_80F0CD4`: curled up; 20 ticks after the entry.
local function curl(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        s.hit = nil
        s.speed = SPEED
        me.anim = ANIM_CURL
        s.timer = CURL_TICKS
    elseif link.run_out(s) then
        link.go(me, LAUNCH)
    end
end

-- `sub_80F0E96`: from the bottom row straight at the enemy (leg 1); else
-- down to it (leg 0).
local function head_off(me: Object, s: State)
    if me.panel_y == BOTTOM_ROW then
        me.vel = Vec3.new(me.facing * s.speed, 0, me.vel.z)
        s.leg = bit32.bor(bit32.band(s.leg, 0xFFFFFF00), 1)
    else
        me.vel = Vec3.new(0, s.speed, me.vel.z)
        s.leg = bit32.band(s.leg, 0xFFFFFF00)
    end
end

-- `sub_80F0D08`: rolling, the riding hit on; 10 ticks after the entry,
-- he sets off.
local function launch(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.collision:set_region(nil)
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me:set_status("moving", true)
        me.anim = ANIM_ROLL
        s.hit = riding_hit.spawn(me, me.attack_element, HIT, link.damage_word(me))
        battle.play_sound(SOUND)
        s.timer = LAUNCH_TICKS
    elseif link.run_out(s) then
        head_off(me, s)
        link.go(me, ROLL)
    end
end

-- `sub_80F0D64`: the roll. Off solid ground he lands; reaching or passing
-- his panel's center along his leg, he snaps to it and follows the route.
local function roll(me: Object, s: State)
    local cx, cy = field.center(me.panel_x, me.panel_y)
    local across = bit32.btest(s.leg, 1)
    local center = if across then cx else cy
    local old, v = me.pos, me.vel
    local x, y = int.i32(old.x + v.x), int.i32(old.y + v.y)
    me.pos = Vec3.new(x, y, old.z)
    local px, py = link.panel_at(x, y)
    if not field.check(px, py, ROLL_REQUIRE, ROLL_FORBID) then
        link.go(me, LAND)
        return
    end
    local was, now = if across then old.x else old.y, if across then x else y
    if center ~= was and link.between(now, was, center) then
        me:set_coordinates_from_panel()
        if route(me, s) then
            link.go(me, LAND)
            return
        end
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- `sub_80F0DEC`: back on his panel, his region back, the riding hit gone;
-- 20 ticks, then idle.
local function land(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_LAND
        me.collision:set_region(regions.single)
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:unreserve_panel(me.panel_x, me.panel_y)
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me:set_status("moving", false)
        s.timer = LAND_TICKS
        local hit = s.hit
        if hit then
            riding_hit.stop(hit)
        end
    elseif link.run_out(s) then
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = { [CURL] = curl, [LAUNCH] = launch, [ROLL] = roll, [LAND] = land }

return define.chip {
    id = "bn6:rslash",
    codes = { "S" },
    element = "null",
    family = "sword",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = { formula = "navi_level", base = 80, per_level = 20 },
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    library = { number = 402, index = 0, sort = 7 },
    slot_in_limit = 1,
    -- `sub_80F0CB0` (`off_80F0CC4`).
    action = define.action {
        id = "bn6:rslash/action",
        state = STATE,
        update = function(me: Object, s: State)
            link.run(me, s, PHASES, "RSlash (sub_80F0CB0)")
        end,
    },
}
