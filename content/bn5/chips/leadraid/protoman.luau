--!strict
-- LeadRaid's ProtoMan (BN5's actor object #0x20, 0x080BE1C8; the code BN6's TwinLdrs grew from: `sub_80BD388` is this
-- handler as it is, the rest reworked), brought by the navi chip controller (the summon table's entry 21) on his
-- user's panel, 10 pixels ahead of it and 6 down; he brings Colonel (./colonel). He appears (4 ticks) and charges (his
-- glow, ./glow, from his 11th tick); on his 55th he picks his target and lets Colonel strike: along his row, the
-- first panel he can stand on with an enemy navi on the panel ahead (0x080BE4F0), else the first with an enemy navi
-- anywhere in the column ahead (0x080BE47C), else the last he can stand on. He blinks there and slashes the panel
-- ahead and those above and below it (0x080BE364); then he waits for Colonel to be done (his animation held the
-- while: his `prevent_anim`, which Colonel's spawn sets and Colonel clears) and leaves, and the controller moves on.
-- Off a panel that isn't solid he waits 10 ticks and leaves.

local attachment = require("../../objects/attachment/attachment")
local colonel = require("./colonel")
local collision = require("../../rules/collision")
local glow = require("./glow")
local navi_chips = require("@exelib/navi-chips/navi")
local regions = require("@exelib/regions")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- Colonel (+0x60).
    colonel: Object?,
    -- His glow while he charges (+0x64).
    glow: Object?,
}

local protoman = {}

local SPRITE = asset.sprite("protoman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_SWING = asset.sound("sword-swing")
-- His sword (the attachment row 3), its animation 1, animating while dimmed.
local SWORD = attachment.look { sprite = asset.sprite("sword"), palette = 0 }
local SWORD_ANIM = 1
-- His slash's effect (the effect table's row 0x16), 16 pixels up.
local SLASH = define.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }
local SLASH_Z = 0x10_0000
-- What he looks for (0x080BE5BC, 0x080BE594: an enemy navi's body), and a panel he can stand on: solid, with no body
-- on it but his side's navi's (0x080BE560, 0x080BE4E8).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local SOLID = 0x10
local NOT_FREE = { [0] = 0x0780_0000, 0x0B80_0000 }
-- His place beside his user's panel (pixels ahead and down), his NameID's attach point 0 (sprite 08-01's: 0, 0), where
-- his glow sits.
local AHEAD, DOWN = 10, 6
local GLOW_X, GLOW_Z = 0, 0
-- When his glow comes, and when he picks his target (counting up from 0).
local GLOW_AT, PICK = 10, 55
-- When his slash lands (ticks left of 21).
local STRIKE = 0x0C

-- 0x080BE59C: whether an enemy navi stands on (x, y).
local function enemy_on(me: Object, x: number, y: number): boolean
    return bit32.btest(field.flags(x, y), ENEMY_NAVI[me.alliance])
end

-- 0x080BE568: whether an enemy navi stands anywhere in column `x`.
local function enemy_in_column(me: Object, x: number): boolean
    for y = 1, 3 do
        if enemy_on(me, x, y) then
            return true
        end
    end
    return false
end

-- 0x080BE4F0 and 0x080BE47C: along his row from his own panel while there is a panel ahead, the first panel he can
-- stand on whose next panel (`column` false) or column (true) holds an enemy navi. Nil for none (0x080BE4F0); the last
-- he could stand on, or his own, for the column's search (0x080BE47C).
local function search(me: Object, column: boolean): (number?, number)
    local y = me.panel_y
    local x = me.panel_x
    local tx = x
    while field.valid(x + me.facing, y) do
        if field.check(x, y, SOLID, NOT_FREE[me.alliance]) then
            tx = x
            local found = if column then enemy_in_column(me, x + me.facing) else enemy_on(me, x + me.facing, y)
            if found then
                return tx, y
            end
        end
        x += me.facing
    end
    if column then
        return tx, y
    end
    return nil, y
end

-- Enter the phase with a timer, then count down (from its first tick); true once it has run out.
local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- Action 0 (0x080BE26C): he appears; 4 ticks; then he charges, or goes off a panel that isn't solid.
local function appear(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
        me.timer2 = 0
    end
    if countdown(me, 4) then
        me:set_action(if field.check(me.panel_x, me.panel_y, SOLID, 0) then 4 else 0x10)
    end
end

-- Action 4 (0x080BE2A6): standing (animation 0), his glow on his 11th tick; on his 55th the glow goes, he picks his
-- target and Colonel may strike.
local function charge(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = 0
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer == GLOW_AT then
        s.glow = glow.spawn(me, GLOW_X, GLOW_Z)
    end
    local t = int.u16(me.timer + 1)
    me.timer = t
    if t < PICK then
        return
    end
    glow.stop(s.glow)
    local x, y = search(me, false)
    if x == nil then
        x, y = search(me, true)
    end
    me.future_panel_x, me.future_panel_y = x :: number, y
    colonel.go(s.colonel)
    me:set_action(8)
end

-- Action 8, phase 0 (0x080BE318): animation 4; 4 ticks.
local function blink_out(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 4) then
        me.phase, me.phase_init = 4, 0
    end
end

-- Action 8, phase 4 (0x080BE33A): on his target's panel (animation 3); 4 ticks.
local function blink_in(me: Object)
    if me.phase_init == 0 then
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:set_coordinates_from_panel()
        me.anim = 3
    end
    if countdown(me, 4) then
        me.phase, me.phase_init = 8, 0
    end
end

-- 0x080BE398: his slash on the panel ahead and those above and below it (no spark, hitting navis, a slash, hit
-- modifier 3, running while dimmed), its effect there, the camera shaking (3, 10 ticks), the screen flashing (10
-- ticks, going on while dimmed).
local function slash(me: Object)
    local x, y = me.panel_x + me.facing, me.panel_y
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = regions.wide,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.slash,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, SLASH_Z), SLASH, bit32.bxor(me.alliance, me.flip))
    battle.shake_camera(3, 0x0A)
    battle.palette_flash(0, 0x0A, true, false)
end

-- Action 8, phase 8 (0x080BE364): his sword out (animation 5) with its sound; at 12 left the slash; 21 ticks; then
-- Colonel may strike (again).
local function strike(me: Object)
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = 5, 0xFF
        attachment.attach(me, SWORD, "related", { anim = SWORD_ANIM, while_dimmed = true })
        battle.play_sound(SOUND_SWING)
        me.timer = 0x15
        me.phase_init = 4
    end
    if me.timer == STRIKE then
        slash(me)
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        colonel.go((me.state :: State).colonel)
        me.phase, me.phase_init = 0x0C, 0
    end
end

-- Action 8, phase 0xC (0x080BE3F2): until Colonel is done (his `prevent_anim` cleared); then he leaves.
local function wait(me: Object)
    if me.prevent_anim == 0 then
        me:set_action(0x0C)
    end
end

local ATTACK = { [0] = blink_out, [4] = blink_in, [8] = strike, [0xC] = wait }

local function attack(me: Object)
    local f = ATTACK[me.phase]
    if not f then
        error(string.format("LeadRaid's ProtoMan: phase %#x reads past his table (0x080BE308)", me.phase))
    end
    f(me)
end

-- Action 0xC (0x080BE404): his sword away (his first related slot cleared), animation 4; 4 ticks; the controller
-- moves on, and he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.related1 = nil
        me.anim = 4
    end
    if countdown(me, 4) then
        -- (The lifecycle byte alone, as the game stores it.)
        local controller = (me.state :: State).controller
        if controller then
            navi_chip.navi_left(controller)
        end
        me.lifecycle = "destroy"
    end
end

-- Action 0x10 (0x080BE430): off a panel that isn't solid, standing (animation 0) 10 ticks; then he leaves.
local function off(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 0x0A) then
        me:set_action(0x0C)
    end
end

local ACTIONS = { [0] = appear, [4] = charge, [8] = attack, [0xC] = leave, [0x10] = off }

-- 0x080BE246.
local function act(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("LeadRaid's ProtoMan: action %#x reads past his table (0x080BE258)", me.action))
    end
    action(me)
end

-- 0x080BE1EC: his sprite (`sub_801BE2A`'s word 0x01030801: 08-01, animation 3, a ground shadow), his palette
-- (0x0800DA8A: navi 1's count by the chip's level, 0), facing his side's way; beside his user's panel on the ground;
-- Colonel; NameID 0x101 (what his glow's place reads); his first update.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 3, 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = 0
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.facing * AHEAD * 0x10000), int.i32(p.y + DOWN * 0x10000), 0)
    local s = me.state :: State
    s.colonel = colonel.spawn(me)
    me:set_lifecycle("update")
    act(me)
end

protoman.kind = define.kind {
    id = "leadraid/protoman",
    pool = "actor",
    state = { controller = "object", colonel = "object", glow = "object" },
    -- 0x080BE1C8.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            act(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BE456: LeadRaid's ProtoMan for `user`, reporting to `controller` (the summon table's entry 21).
function protoman.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(protoman.kind, user, spec)
    if not o then
        return nil
    end
    (o.state :: State).controller = controller
    return o
end

return protoman
