--!strict
-- Roll, the navi of the Roll chips (the original's actor object #0x2C,
-- `sub_80C07BC`), brought by their navi chip controller. She fades in over
-- her user's panel; warps up and down in front of the nearest enemy navi
-- ahead (`sub_80C0C9A`); whips it three times (a hit region 10 ticks
-- apart, with her ribbon out); warps up and away; warps her user back in
-- herself; and drops a heart on her user's panel (./heart) that heals
-- three times her damage. Then she goes, and the controller moves on. With
-- no enemy navi ahead she leaves at once. (The controller skips the user's
-- warp back in for her: she brings it herself.) See docs/engine/chips.md
-- §3.6.20.

local attachment = require("../../objects/attachment/attachment")
local heart = require("./heart")
local panels = require("../../lib/panels")
local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

-- What a chip summons her with (the chips' first parameter picks them):
-- her whip's rounds of three hits (1; 3 for the parameter 5, which no chip
-- has) and her palette (Roll3's is 1).
export type Summon = { rounds: number, palette: number }

export type State = {
    -- Her whip's rounds and her palette (from Param1).
    whip_rounds: number,
    palette: number,
    -- Where she lands (Param3, Param4).
    target_x: number,
    target_y: number,
    -- Her user's panel when the chip was used (object +0xC, +0xD), where
    -- her heart falls.
    home_x: number,
    home_y: number,
    -- The gray of her color shader, fading in (object +0x19).
    gray: number,
    -- Whip rounds left (ExtraVars[0]), and her whip's sound counter
    -- (ExtraVars[2]).
    rounds: number,
    swish: number,
    -- The controller waiting for her (its pointer sits in ExtraVars[1]).
    controller: Object?,
    -- Her ribbon (ExtraVars[3], the attachment's slot).
    held: Object?,
}

local roll = {}

local SPRITE = asset.sprite("roll-heart")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_ROLL = asset.sound("roll")
local SOUND_ROLL_2 = asset.sound("roll-2")

-- The controller calls her spawner through r3, which her position takes
-- (her init sets it all).
local SPAWNER_ADDRESS = 0x080C0DD9

-- Her fade-in's starting gray, and her warp's sparkle.
local FADE_GRAY = 0x1F
local SPARKLE = heart.sparkle

-- What she looks for (`off_80C0D90`: an enemy navi's body), and what a
-- panel she lands on must be free of (as words).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local OCCUPIED = 0x0F80_0000

-- How high she warps (Z's whole part), and how fast (16.16).
local WARP_HEIGHT = 0xA0
local WARP_SPEED = 0xF_0000

-- Her ribbon (the original's attachment row 0x12), its animation 5,
-- animating while dimmed.
local RIBBON = attachment.look { sprite = asset.sprite("roll-heart"), palette = 0 }
local RIBBON_ANIM = 5

-- A sparkle 20 pixels above her.
local function sparkle_above(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x14_0000), SPARKLE)
end

-- `sub_80C0DC2`: a sparkle around her (one RNG draw).
local function sparkle_around(me: Object)
    battle.effect(battle.jitter(0x10, me.pos), SPARKLE)
end

-- `sub_80C0D98`: a hit on (x, y), 20 pixels up, running while dimmed
-- (`sub_80C53A6`).
local function whip(me: Object, x: number, y: number)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 0,
        z = 0x14_0000,
        region = regions.single,
        hit_effect = sparks.plain,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- `sub_80C0C9A`: where she lands: in front of the nearest enemy navi ahead
-- (on a free panel, or her user's), nearest column first. Its tie-break
-- compares the candidate's row with the best's row distance, as the game
-- does. 0 if none.
local function target(me: Object): (number, number)
    local found = panels.filtered_except(me, ENEMY_NAVI[me.alliance], 0)
    if #found == 0 then
        return 0, 0
    end
    local user = me.related1 :: Object
    local right = bit32.bxor(me.alliance, me.flip) == 0
    local candidates = {}
    for _, p in found do
        local x, y = panels.x(p), panels.y(p)
        local ahead = if right then x > me.panel_x else me.panel_x > x
        if ahead then
            local lx = x - me.facing
            if (lx == user.panel_x and y == user.panel_y) or field.check(lx, y, 0, OCCUPIED) then
                table.insert(candidates, { lx, y })
            end
        end
    end
    local bx, by, bdx, bdy = 0, 0, 0xFF, 0xFF
    for _, c in candidates do
        local x, y = c[1], c[2]
        local dx, dy = math.abs(x - me.panel_x), math.abs(y - me.panel_y)
        local take = false
        if dx < bdx then
            take = true
        elseif dx == bdx then
            if y < bdy then
                take = true
            elseif y == bdy then
                take = y < by
            end
        end
        if take then
            bx, by, bdx, bdy = x, y, dx, dy
        end
    end
    return bx, by
end

-- Enter the phase with a timer (PhaseInitialized), or count down; true
-- once the timer is 0 or below.
local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

-- Action 0 (`sub_80C0870`): appear.
-- `sub_80C088C`: a sparkle; she fades in from gray 31, 2 a tick,
-- flickering.
local function fade_in(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        sparkle_above(me)
        s.gray = FADE_GRAY
        battle.play_sound(SOUND_APPEAR)
        me.timer = 0
        me.phase_init = 4
        return
    end
    me.visible = true
    local t = me.timer + 1
    me.timer = t
    if bit32.band(t, 1) ~= 0 then
        me.visible = false
    end
    local gray = s.gray - 2
    s.gray = gray
    if gray > 0 then
        return
    end
    me.visible = true
    s.gray = 0
    me.phase, me.phase_init = 4, 0
end

-- `sub_80C08EE`: 15 ticks.
local function settle(me: Object)
    if countdown(me, 0xF) then
        me:set_action(4)
    end
end

-- Action 4 (`sub_80C0912`): warp to her target.
-- `sub_80C0938`: find it, or leave.
local function aim(me: Object)
    local x, y = target(me)
    if x == 0 then
        me:set_action(0xC)
        return
    end
    local s = me.state :: State
    s.target_x, s.target_y = x, y
    me.phase, me.phase_init = 4, 0
end

-- `sub_80C0956` / `sub_80C0B90`: animation 2, her warp's sound; 5 ticks, then
-- hidden (true).
local function vanish(me: Object): boolean
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = 2, 0
        battle.play_sound(SOUND_ROLL)
        me.timer = 5
        me.phase_init = 4
        return false
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return false
    end
    me.visible = false
    return true
end

local function warp_out(me: Object)
    if vanish(me) then
        me.phase, me.phase_init = 8, 0
    end
end

-- `sub_80C0988` / `sub_80C0BC2`: rise until 160 pixels up, sparkling every
-- 5 ticks; true once up.
local function rise(me: Object): boolean
    if me.phase_init == 0 then
        me.timer = 4
        me.vel = Vec3.new(me.vel.x, me.vel.y, WARP_SPEED)
        me.phase_init = 4
    else
        local p = me.pos
        local z = p.z + me.vel.z
        me.pos = Vec3.new(p.x, p.y, z)
        if int.asr(me.pos.z, 16) >= WARP_HEIGHT then
            return true
        end
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me.timer = 5
        sparkle_around(me)
    end
    return false
end

local function warp_up(me: Object)
    if rise(me) then
        me.phase, me.phase_init = 0xC, 0
    end
end

-- `sub_80C09C6`: onto her target's panel, and down, sparkling every 3
-- ticks.
local function warp_down(me: Object)
    if me.phase_init == 0 then
        me.timer = 1
        local v = me.vel
        me.vel = Vec3.new(v.x, v.y, -v.z)
        local s = me.state :: State
        me.panel_x, me.panel_y = s.target_x, s.target_y
        me:set_coordinates_from_panel()
        me.phase_init = 4
    else
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, p.z + me.vel.z)
        if int.asr(me.pos.z, 16) <= 0 then
            me.pos = Vec3.new(p.x, p.y, 0)
            me.phase, me.phase_init = 0x10, 0
            return
        end
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me.timer = 3
        sparkle_around(me)
    end
end

-- `sub_80C0A0E`: shown again with a sparkle; 5 ticks.
local function arrive(me: Object)
    if me.phase_init == 0 then
        me.timer = 5
        sparkle_above(me)
        me.visible = true
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(8)
    end
end

-- Action 8 (`sub_80C0A4C`): the whipping.
-- `sub_80C0A6C`: animation 3; 9 ticks.
local function wind_up(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
    end
    if countdown(me, 9) then
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80C0A90`: her ribbon out; a hit on the panel ahead every 10 ticks,
-- three to a round (a round, or as many as her chip gives), the whip swishing
-- every 8 ticks; the animation restarts each round.
local function lash(me: Object)
    local s = me.state :: State
    local x, y = me.panel_x + me.facing, me.panel_y
    if me.phase_init == 0 then
        local ribbon = attachment.attach(me, RIBBON, "held", { anim = RIBBON_ANIM, while_dimmed = true })
        if ribbon then
            ribbon.run_while_dimmed = true
        end
        me.timer = 0xA
        me.timer2 = 3
        s.rounds = s.whip_rounds
        whip(me, x, y)
        me.phase_init = 4
        return
    end
    local swish = bit32.band(s.swish + 1, 7)
    s.swish = swish
    if swish == 1 then
        battle.play_sound(SOUND_ROLL_2)
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.timer = 0xA
    local hits = me.timer2 - 1
    me.timer2 = hits
    if hits > 0 then
        whip(me, x, y)
        return
    end
    local rounds = s.rounds - 1
    s.rounds = rounds
    if rounds == 0 then
        me.phase, me.phase_init = 8, 0
        return
    end
    me.timer2 = 3
    whip(me, x, y)
    me.anim_loaded = 0xFF
end

-- `sub_80C0B4A`: her ribbon away (its slot cleared); 10 ticks.
local function rest(me: Object)
    if me.phase_init == 0 then
        me.anim = 0;
        (me.state :: State).held = nil
    end
    if countdown(me, 0xA) then
        me:set_action(0xC)
    end
end

-- Action 0xC (`sub_80C0B76`): warp away.
local function leave_out(me: Object)
    if vanish(me) then
        me.phase, me.phase_init = 4, 0
    end
end

local function leave_up(me: Object)
    if rise(me) then
        me:set_action(0x10)
    end
end

-- Action 0x10 (`sub_80C0C04`): the heal.
-- `sub_80C0C20`: 10 ticks, then her user warps back in.
local function recall(me: Object)
    if countdown(me, 0xA) then
        navi_chip.warp(me.related1 :: Object, false)
        me.phase, me.phase_init = 4, 0
    end
end

-- `sub_80E199A`: the heal: three times her damage (with its double flag,
-- six).
local function heal_amount(damage: number): number
    local d = bit32.band(damage, 0x87FF)
    local n = if bit32.band(d, 0x8000) ~= 0 then bit32.band(d, 0x7FFF) * 2 else bit32.band(d, 0x7FFF)
    return n * 3
end

-- `sub_80C0C48`: her heart on her user's panel; 90 ticks, then she goes.
local function drop_heart(me: Object)
    if me.phase_init == 0 then
        local s = me.state :: State
        heart.spawn(me.related1 :: Object, s.home_x, s.home_y, heal_amount(me.damage))
    end
    if countdown(me, 0x5A) then
        me:set_lifecycle("destroy")
    end
end

local PHASES = {
    [0] = { [0] = fade_in, [4] = settle },
    [4] = { [0] = aim, [4] = warp_out, [8] = warp_up, [0xC] = warp_down, [0x10] = arrive },
    [8] = { [0] = wind_up, [4] = lash, [8] = rest },
    [0xC] = { [0] = leave_out, [4] = leave_up },
    [0x10] = { [0] = recall, [4] = drop_heart },
}

-- `sub_80C0840`: the action's phase; her sprite; her color shader from
-- the gray (`sub_80C0DB0`).
local function update_state(me: Object)
    local phases = PHASES[me.action]
    if not phases then
        error(string.format("Roll action %#x reads past her table (off_80C0858)", me.action))
    end
    local f = phases[me.phase]
    if not f then
        error(string.format("Roll action %#x phase %#x reads past its table", me.action, me.phase))
    end
    f(me)
    me:update_sprite()
    local gray = (me.state :: State).gray
    me.sprite.color_shader = gray * 0x421
end

-- `sub_80C07DC`: over her user's panel, faded out (gray), in her chip's
-- palette; then her first update at once.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = (me.state :: State).palette
    sprite.color_shader = 0x7FFF
    me:set_lifecycle("update")
    update_state(me)
end

-- `sub_80C0C8C`: the controller moves on, and she is freed.
local function finish(me: Object)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:free()
end

roll.kind = define.kind {
    id = "roll/navi",
    pool = "actor",
    state = {
        home_x = "u8",
        home_y = "u8",
        gray = "u8",
        rounds = "u8",
        swish = "u8",
        controller = "object",
        held = "object",
        whip_rounds = "u8",
        palette = "u8",
        target_x = "u8",
        target_y = "u8",
    },
    -- `sub_80C07BC`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            update_state(me)
        else
            finish(me)
        end
    end,
}

-- `sub_80C0DD8`: the Roll chips' navi hook (`off_802CD5C[0]`): Roll for
-- `user`, reporting to `controller`. (Unlike the others she takes no
-- element.)
function roll.summon(args: Summon): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = battle.spawn(roll.kind, Vec3.new(spec.panel_y, spec.element, SPAWNER_ADDRESS))
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
        local s = o.state :: State
        s.home_x, s.home_y = spec.panel_x, spec.panel_y
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(spec.damage)
        o.related1 = user
        s.controller = controller
        s.whip_rounds = args.rounds
        s.palette = args.palette
        return o
    end
end

return roll
