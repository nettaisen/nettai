--!strict
-- AquaMan's water gun (attack object #0x33, 0x080D3468: EXE4's own), set on the panel ahead of him: on a solid panel
-- nothing stands on (else gone at once in a puff), it sets a region on its panel (no collision types, never
-- presented) and pops up (21 ticks), stands 20, readies 25, then sprays: a hit on the panel ahead of it (9 ticks),
-- then two on the panel past that (7 each); 10 ticks on it sinks (5) and, 21 later, goes in a puff. Going, it empties
-- AquaMan's slot for it.
--
-- (While it readies it counts the presses of A its side's navi makes while dimmed, and picks a strength of 1 to 3 from
-- them, which nothing reads: 0x080D362C loads it and sprays twice.)

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local rock = require("../../lib/rock")
local sparks = require("../../lib/sparks")

local water_gun = {}

-- What AquaMan keeps it in (his +0x64, the word its +0x60 points at).
export type Holder = { gun: Object? }

local SPRITE = asset.sprite("water-gun")
local SOUND_UP = asset.sound("tops")
local SOUND_SPRAY = asset.sound("water-gun")
local SOLID = 0x10
-- Bodies, neutral objects, blockers and reservations (0x080D351A).
local OCCUPIED = 0x0F88_0080
-- Its hits' height.
local SPRAY_Z = 0x0A_0000

local UP, STAND, READY, SPRAY, SINK, GONE = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14

-- Enter the phase with a timer, or count down; true once it has run out: past 0 (`bge`), or at 0 (`bgt`).
local function countdown(me: Object, ticks: number, past: boolean): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return if past then left < 0 else left <= 0
end

-- It goes: a puff `dz` up from it (effect row 0x15), AquaMan's slot emptied.
local function leave(me: Object, dz: number)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + dz), rock.effects.puff)
    local owner = me.related1
    if owner then
        (owner.state :: Holder).gun = nil
    end
    me:set_lifecycle("destroy")
end

-- A hit `dx` panels ahead (`sub_80C53A6`: single, the aqua spark, an attack of its element at navis 10 pixels up,
-- hit modifier 3), resolving while dimmed.
local function spray(me: Object, dx: number)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x + dx * me.facing,
        panel_y = me.panel_y,
        element = me.element,
        z = SPRAY_Z,
        region = regions.single,
        hit_effect = sparks.aqua,
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

-- Action 0 (0x080D3504): on a solid panel nothing stands on, its region on it and its sound; 20 ticks. Else gone.
local function up(me: Object)
    if me.phase_init == 0 then
        if not field.solid(me.panel_x, me.panel_y) or bit32.btest(field.flags(me.panel_x, me.panel_y), OCCUPIED) then
            leave(me, 0)
            return
        end
        me.collision:set_region(regions.single)
        battle.play_sound(SOUND_UP)
    end
    if countdown(me, 20, true) then
        me:set_action(STAND)
    end
end

-- Action 4 (0x080D355E): animation 1, 20 ticks.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 1
    end
    if countdown(me, 20, false) then
        me:set_action(READY)
    end
end

-- Action 8 (0x080D3584): animation 5, 25 ticks.
local function ready(me: Object)
    if me.phase_init == 0 then
        me.anim = 5
    end
    if countdown(me, 25, false) then
        me:set_action(SPRAY)
    end
end

-- Action 0xC (0x080D35D0), phase 0 (0x080D35F0): animation 2, a hit on the panel ahead with its sound; 8 ticks. Phase
-- 4 (0x080D363C): animation 3, a hit on the panel past it; 7 ticks; twice. Phase 8 (0x080D368C): 10 ticks.
local function sprays(me: Object)
    if me.phase == 0 then
        if me.phase_init == 0 then
            me.anim = 2
            spray(me, 1)
            battle.play_sound(SOUND_SPRAY)
        end
        if countdown(me, 8, true) then
            me.timer2 = 2
            me.phase, me.phase_init = 4, 0
        end
    elseif me.phase == 4 then
        if me.phase_init == 0 then
            me.anim = 3
            spray(me, 2)
        end
        if countdown(me, 7, false) then
            me.timer2 = int.u16(me.timer2 - 1)
            if int.i16(me.timer2) > 0 then
                me.phase_init = 0
            else
                me.phase, me.phase_init = 8, 0
            end
        end
    elseif me.phase == 8 then
        if countdown(me, 10, false) then
            me:set_action(SINK)
        end
    else
        error(string.format("AquaMan's water gun's phase %#x reads past its table (0x080D35E4)", me.phase))
    end
end

-- Action 0x10 (0x080D36B4): animation 4, 5 ticks.
local function sink(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 4, true) then
        me:set_action(GONE)
    end
end

-- Action 0x14 (0x080D36DA): 21 ticks; then gone, its puff 16 pixels up.
local function gone(me: Object)
    if countdown(me, 20, true) then
        leave(me, 0x10_0000)
    end
end

local ACTIONS: { [number]: (Object) -> () } = {
    [UP] = up,
    [STAND] = stand,
    [READY] = ready,
    [SPRAY] = sprays,
    [SINK] = sink,
    [GONE] = gone,
}

local function tick(me: Object)
    local f = ACTIONS[me.action]
    if not f then
        error(string.format("AquaMan's water gun's action %#x reads past its table (0x080D34EC)", me.action))
    end
    f(me)
end

-- 0x080D348C: its sprite (10-10, no shadow), shown, in animation 0; on its panel, facing its side's way; its collision
-- (no types: it only holds its panel); then its first tick.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0xFF
    me:set_coordinates_from_panel()
    sprite:set_flip(me.alliance)
    if not me:create_collision() then
        me:free()
        return
    end
    me:set_lifecycle("update")
    tick(me)
end

water_gun.kind = new.kind {
    id = "aquaman/water-gun",
    pool = "attack",
    -- 0x080D3468: by its state, then its sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080D3710: a water gun on (x, y) of AquaMan's side, his, with `element` and the damage word, running while dimmed,
-- in his slot. (Its position is the spawner's registers: the row, the element and 0.)
function water_gun.spawn(aquaman: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(water_gun.kind, Vec3.new(y, element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o.alliance = aquaman.alliance
    o:set_damage_word(damage)
    o.run_while_dimmed = true
    o.related1 = aquaman
    ;(aquaman.state :: Holder).gun = o
    return o
end

return water_gun
