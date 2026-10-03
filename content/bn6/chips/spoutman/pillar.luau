--!strict
-- The water spouting under SpoutMan (the original's effect object #0x2D,
-- `sub_80E37F4`): it rises (9 ticks), then plays in step with him, who
-- signals it through its phase_init byte (1: spout; 2: done); it shows
-- while he does. Its spawner keeps it in a slot of its owner's, which it
-- clears when it goes (if it is still there). The navi chip's (the
-- original's Param1 4) acts and steps its sprite while dimmed; another
-- (his own AI's) waits a dimming out and goes as soon as its owner isn't
-- in action 0xB. The fraction of its Z is its spawner's register garbage
-- (the object loop's count of the objects of the one before him, times
-- 4), which its init keeps (compat's `scratch_z_fraction`). See
-- docs/engine/chips.md §3.6.11.

local pillar = {}

export type State = {
    -- The owner whose `pillar` state field holds it (the game's pointer to
    -- that slot, in its ExtraVars[0]).
    slot_owner: Object?,
    -- It acts, and animates, while the battle is dimmed (Param1 4).
    while_dimmed: boolean,
}

-- The owner's state: the slot this pillar sits in.
export type OwnerState = { pillar: Object? }

local SPRITE = asset.sprite("bn6:spout-pillar")

-- The owner's action a navi AI's pillar lasts through.
local OWNER_ACTION = 0xB

-- The signals its owner writes to its phase_init.
local SPOUT, DONE = 1, 2

-- `sub_80E3824`: on its panel on the ground.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80E38C8`: 9 ticks rising (the first counted on the tick it
-- starts), then standing (animation 3).
local function rise(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = 9
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.anim = 3
    me:set_action(4)
end

-- `sub_80E38EE`: stand until the owner signals a spout.
local function stand(me: Object)
    if me.phase_init == SPOUT then
        me.anim = 1
        me:set_action(8)
    end
end

-- `sub_80E3904`: spout (animation 1) for 27 ticks, then fall back
-- (animation 2).
local function spout(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 1
        me.timer = 0x1B
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.anim = 2
    me:set_action(0xC)
end

-- `sub_80E3930`: fallen: done, or on to standing again at the next spout.
local function fallen(me: Object)
    local signal = me.phase_init
    if signal == DONE then
        me:set_lifecycle("destroy")
    elseif signal == SPOUT then
        me.anim = 3
        me:set_action(0x10)
    end
end

-- `sub_80E3950`: standing again, until the next spout.
local function again(me: Object)
    if me.phase_init == SPOUT then
        me:set_action(8)
    end
end

local ACTIONS = { [0] = rise, [4] = stand, [8] = spout, [0xC] = fallen, [0x10] = again }

-- `sub_80E386A`: gone once (not the navi chip's) the owner is out of
-- action 0xB; shown while the owner is; outside a dimming unless it is the
-- navi chip's.
local function tick(me: Object)
    local own = (me.state :: State).while_dimmed
    local owner = me.related1 :: Object
    if not own and owner.action ~= OWNER_ACTION then
        me:set_lifecycle("destroy")
        return
    end
    me:copy_visibility(owner)
    if not own and battle.dimmed() then
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("SpoutMan's pillar action %#x reads past its table (off_80E38B4)", me.action))
    end
    action(me)
end

-- `sub_80E3962`: out of its owner's slot if it is still there, and free.
local function finish(me: Object)
    local owner = (me.state :: State).slot_owner
    if owner then
        local s = owner.state :: OwnerState
        if s.pillar == me then
            s.pillar = nil
        end
    end
    me:free()
end

pillar.kind = define.kind {
    id = "bn6:spoutman/pillar",
    pool = "effect",
    state = { slot_owner = "object", while_dimmed = "bool" },
    -- `sub_80E37F4`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            finish(me)
        end
        if (me.state :: State).while_dimmed then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80E3976`: a pillar on (x, y) for `owner`, kept in `owner`'s
-- `pillar` state field; `while_dimmed`: the navi chip's.
function pillar.spawn(owner: Object, x: number, y: number, while_dimmed: boolean): Object?
    local o = battle.spawn(pillar.kind, Vec3.new(y, 0, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.related1 = owner
        local s = o.state :: State
        s.while_dimmed = while_dimmed
        s.slot_owner = owner;
        (owner.state :: OwnerState).pillar = o
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return pillar
