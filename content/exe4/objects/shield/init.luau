--!strict
-- EXE4's guard shield (attack object #0x2B, 0x080D2224; EXE6's Reflectors' `sub_80C96A0`, with EXE4's differences):
-- the shield a navi holds up while it guards. Visual only: the guard is the navi's own status. It follows its owner
-- at the offset it was spawned with, shown as its owner is, and fades once the owner clears the slot it was stored in
-- (its first related object). Unlike EXE6's: one sheet (0c-1b), its look by its row (animation 2 x the row, the fade
-- the next, 14 ticks for every row), its sound 0xC1, running while dimmed (not while paused) and animating then too,
-- with no test of its owner's having vanished.

local slot = require("@exelib/slot")

local shield = {}

export type State = {
    slot: Slot,
    -- Its look's row (its second parameter): 1 the Reflect program's, 2 the Shield program's.
    row: number,
}

local SPRITE = asset.sprite("reflector-shield")
local SOUND = asset.sound("shield")
local FADE_TICKS = 14

-- The actions: held up, then fading.
local UP, FADING = 0, 4

-- 0x080D22CC: at the owner's position plus the offset kept in the velocity, shown as the owner is; once the slot is
-- cleared, fade.
local function follow(me: Object)
    local owner = me.related1 :: Object
    me.pos = owner.pos + me.vel
    me:copy_visibility(owner)
    if not slot.occupied(owner, (me.state :: State).slot) then
        me.action = FADING
        me.phase = 0
        me.phase_init = 0
    end
end

-- 0x080D230A: the fade (the row's next animation), 14 ticks, then gone.
local function fade(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        me.anim = 2 * (me.state :: State).row + 1
        me.timer = FADE_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:set_lifecycle("destroy")
    end
end

-- 0x080D2298: the action, then shown unless the viewer is blind to its side.
local function tick(me: Object)
    local a = me.action
    if a == UP then
        follow(me)
    elseif a == FADING then
        fade(me)
    else
        error(string.format("the guard shield's action %#x reads past 0x080D22C4", a))
    end
    me.visible = true
    me:hide_from_blind()
end

-- 0x080D2248: the look (its row's animation), the panel under the offset (the spawn position), the sound, and the
-- offset kept in the velocity.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 2 * (me.state :: State).row
    me.anim_loaded = 0xFF
    me:set_panel_from_coordinates()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    battle.play_sound(SOUND)
    me.vel = me.pos
    me:set_lifecycle("update")
    tick(me)
end

shield.kind = new.kind {
    id = "guard-shield",
    pool = "attack",
    state = { slot = slot.TYPE, row = "u8" },
    -- 0x080D2224: the state, then the sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080D2338: a shield of look row `row` for `owner`, `offset` from it (in the spawn position); stored in the
-- owner's first related object, unless the pool is full; it runs while dimmed.
function shield.spawn(owner: Object, offset: Vec3, row: number): Object?
    local o = battle.spawn(shield.kind, offset)
    if o then
        o.related1 = owner
        local s = o.state :: State
        s.slot, s.row = "related", row
        slot.fill(owner, "related", o)
        o.alliance = owner.alliance
        o.run_while_dimmed = true
    end
    return o
end

return shield
