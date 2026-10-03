--!strict
-- BN5's BigNoise (the navi chips' action with the summon table's entry 20, 0x080BD702): BN5's own ShadeMan (actor
-- object #0x1B, 0x080BD510), brought by the navi chip controller. He appears (4 ticks), stands 10, cries out
-- (animation 5) for 15, and sends his noise (attack object #0x04, 0x080C4DE0, BN5's own: its Param2 1) from the panel
-- ahead; 40 ticks later he lowers (animation 7) for 11, stands 20, leaves (animation 4) in 4 and goes, and the
-- controller moves on.
--
-- The noise, ShadeMan's image (08-0d's animation 12) 40 pixels ahead of its panel, hits navis (piercing, or a plain
-- attack in a battle of mode 1, a netbattle) over the three columns ahead of it and its own (region 0x1B: with its
-- Param2, no hit modifier and paralysis for 150 ticks), highlighting them, for 60 ticks. (Without its Param2, the
-- one BN5's ShadeMan the virus sends, it starts on its panel with hit modifier 3 and grows: the fan at 2 ticks, two
-- columns at 4, three at 6, four at 8.)

local collision = require("../../rules/collision")
local navi_chips = require("@common/navi-chips/navi")
local panels = require("@common/panels")
local regions = require("@common/regions")
local statuses = require("../../rules/status")

local bignoise = {}

local SPRITE = asset.sprite("bn5:shademan")

-- ---- The noise ------------------------------------------------------------------------------------------------------

-- Regions 0x1B and 0x1C (the region table's rows: the deep fan and one or two columns more).
local THREE_COLUMNS = define.region {
    panels = { { 0, 0 }, { 1, -1 }, { 1, 0 }, { 1, 1 }, { 2, -1 }, { 2, 0 }, { 2, 1 }, { 3, -1 }, { 3, 0 }, { 3, 1 } },
}
local FOUR_COLUMNS = define.region {
    panels = {
        { 0, 0 },
        { 1, -1 },
        { 1, 0 },
        { 1, 1 },
        { 2, -1 },
        { 2, 0 },
        { 2, 1 },
        { 3, -1 },
        { 3, 0 },
        { 3, 1 },
        { 4, -1 },
        { 4, 0 },
        { 4, 1 },
    },
}
-- The growth of the one without Param2 (0x080C4F14): (its tick, its region).
local GROWTH: { { any } } = {
    { 2, regions.fan },
    { 4, regions.deep_fan },
    { 6, THREE_COLUMNS },
    { 8, FOUR_COLUMNS },
}
local SOUND = asset.sound("bn5:elec-pulse")
local TICKS, AHEAD = 60, 40

export type NoiseState = {
    -- Its Param2: the navi chip's.
    navi: boolean,
    -- The region it has (the collision's region byte, for the highlight).
    region: Region?,
}

local function set_region(me: Object, r: Region?)
    me.collision:set_region(r)
    ;(me.state :: NoiseState).region = r
end

-- 0x080C4EE4: the one without Param2 grows its region at its ticks (while it has one).
local function grow(me: Object, s: NoiseState)
    if s.navi or s.region == nil then
        return
    end
    for _, g in GROWTH do
        if me.timer == g[1] then
            set_region(me, g[2])
            return
        end
    end
end

-- 0x080C4E6A: a hit takes its region off; 60 ticks, growing and highlighting its region (by its side); then gone.
local function noise_tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        set_region(me, nil)
    end
    local s = me.state :: NoiseState
    if me.phase_init == 0 then
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer >= TICKS then
        set_region(me, nil)
        me:set_lifecycle("destroy")
    else
        me.timer += 1
        grow(me, s)
        local r = s.region
        if r then
            panels.highlight_region(me.panel_x, me.panel_y, r, panels.side_direction(me.alliance))
        end
    end
    c:present()
end

-- 0x080C4E04: its look and collision (piercing, a plain attack in a mode 1 battle; hitting navis, hit modifier 3; the
-- navi chip's over region 0x1B, no hit modifier, paralysis 150); shown 40 pixels ahead of its panel; the sound; the
-- first update at once.
local function noise_init(me: Object)
    local s = me.state :: NoiseState
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 12, 0xFF
    if not me:create_collision() then
        me:free()
        return
    end
    local is = if battle.mode() == 1 then collision.attack else collision.piercing
    me:setup_collision(is, collision.hits_navis, 3)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    s.region = regions.single
    if s.navi then
        set_region(me, THREE_COLUMNS)
        me.collision.hit_mod_base = 0
        me.collision.status_base = statuses.paralyze_150
    end
    me.collision:present()
    me.visible = true
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + int.shl(me.facing * AHEAD, 16)), p.y, p.z)
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    noise_tick(me)
end

bignoise.noise = define.kind {
    id = "bn5:bignoise/noise",
    pool = "attack",
    state = { navi = "bool", region = "region" },
    -- 0x080C4DE0: by its lifecycle, then a sprite step.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            noise_init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            noise_tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- ---- ShadeMan -------------------------------------------------------------------------------------------------------

export type State = {
    -- The controller waiting for him (the flag it gave him, his +0x54).
    controller: Object?,
}

local SOUND_APPEAR = asset.sound("bn5:appear")

-- Enter the phase with a timer, then count down (the same tick); true once it has run out.
local function countdown(me: Object, anim: number?, ticks: number): boolean
    if me.phase_init == 0 then
        if anim then
            me.anim = anim
        end
        me.timer = ticks
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

local function next_phase(me: Object, phase: number)
    me.phase, me.phase_init = phase, 0
end

-- The attack's phases (0x080BD5F2: 0x080BD604's table).
local function attack(me: Object)
    local p = me.phase
    if p == 0 then
        -- 0x080BD618: he cries out, 15 ticks.
        if countdown(me, 5, 15) then
            next_phase(me, 4)
        end
    elseif p == 4 then
        -- 0x080BD63A: a tick.
        if countdown(me, nil, 0) then
            next_phase(me, 8)
        end
    elseif p == 8 then
        -- 0x080BD658: the noise from the panel ahead (running while dimmed); 40 ticks.
        if me.phase_init == 0 then
            local o = battle.spawn(bignoise.noise, Vec3.new(me.panel_y, me.element, 0))
            if o then
                o.panel_x, o.panel_y = me.panel_x + me.facing, me.panel_y
                o.element = me.element
                o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
                o.alliance, o.flip = me.alliance, me.flip
                o.related1 = me
                o.run_while_dimmed = true
                ;(o.state :: NoiseState).navi = true
            end
            -- (And his +0x1A byte cleared, which nothing here reads.)
        end
        if countdown(me, nil, 40) then
            next_phase(me, 0xC)
        end
    elseif p == 0xC then
        -- 0x080BD692: he lowers, 11 ticks.
        if countdown(me, 7, 11) then
            next_phase(me, 0x10)
        end
    elseif p == 0x10 then
        -- 0x080BD6B4: he stands, 20 ticks.
        if countdown(me, 0, 20) then
            me:set_action(0xC)
        end
    else
        error(string.format("BigNoise's ShadeMan: phase %#x reads past 0x080BD604", p))
    end
end

-- 0x080BD578: by his action (no end of the battle cuts him short).
local function update(me: Object)
    local a = me.action
    if a == 0 then
        -- 0x080BD59C: he appears, 4 ticks.
        if me.phase_init == 0 then
            battle.play_sound(SOUND_APPEAR)
            me.visible = true
        end
        if countdown(me, nil, 4) then
            me:set_action(4)
        end
    elseif a == 4 then
        -- 0x080BD5CC: he stands, 10 ticks.
        if countdown(me, 0, 10) then
            me:set_action(8)
        end
    elseif a == 8 then
        attack(me)
    elseif a == 0xC then
        -- 0x080BD6DA: he leaves (animation 4), 4 ticks; the controller moves on, and he goes.
        if countdown(me, 4, 4) then
            local controller = (me.state :: State).controller
            if controller then
                navi_chip.navi_left(controller)
            end
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("BigNoise's ShadeMan: action %#x reads past 0x080BD58C", a))
    end
end

bignoise.kind = define.kind {
    id = "bn5:bignoise/navi",
    pool = "actor",
    state = { controller = "object" },
    -- 0x080BD510.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080BD534: 08-0d's animation 3 with a ground shadow, facing his way, on his panel on the ground.
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "ground"
            me.anim, me.anim_loaded = 3, 3
            sprite:set_animation(3)
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, 0)
            me:set_lifecycle("update")
        elseif l == "update" then
            update(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BD702: BigNoise's ShadeMan for `user`, reporting to `controller`.
function bignoise.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(bignoise.kind, user, spec)
    if not o then
        return nil
    end
    (o.state :: State).controller = controller
    return o
end

return bignoise
