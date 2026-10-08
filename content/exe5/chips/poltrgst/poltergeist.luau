--!strict
-- EXE5's Poltrgst (the dimming handler's subtype 49): EXE5's own. Its controller (effect object #0x72, 0x080E8C1C):
-- the usual dimming phases, whose effect (0x080E8C60), when the field has an obstacle Poltergeist takes, sends out a
-- stand-in of the user (actor object #0x58, 0x080C38E8: drawn as the user in animation 12 while the user vanishes),
-- which raises the poltergeist (effect object #0x73, 0x080E8CFC: @exelib/poltergeist, which EXE4's JunkMan shares):
-- it takes every such obstacle (the field-object registry's eight slots in order, but those whose identities aren't
-- `throwable`: EXE5's NameIDs 0xDA, 0xD3, 0xD2, 0xE5, 0xE4 and 0xE7), front to back as its side faces (on one column,
-- top row first), and throws each at a panel (`obstacle.throw`, `sub_800F6AC`: the obstacle rises, shakes 30 ticks,
-- 20 more for each before it, and flies onto the panel, breaking there with a hit of the chip's damage); the panel a
-- random one with the other side's navi on it, else one of the other side's but its own, else any (`sub_800F6C6`, a
-- GetPositiveSignedRNG2 draw each), or, by its first parameter (the chip's: 0 for Poltrgst), the nearest such in front
-- (0x080E8E7C). Once the field has none left, 30 ticks later it ends, and the stand-in gives the user back.

local common = require("@exelib/poltergeist")
local dimming_chips = require("@exelib/dimming")

local poltergeist = {}

local POLTERGEIST = common.make("poltrgst/poltergeist")

-- ---- The stand-in --------------------------------------------------------------------------------------------------

export type StandInState = {
    -- The poltergeist runs (+0x0C, which it clears).
    busy: boolean,
    -- Its controller, waiting on it (its +0x60 pointer to the controller's flag).
    controller: Object?,
    nearest: boolean,
}

local STAND_IN_ANIM = 12

-- 0x080C39CA: the poltergeist out; when it is done, the parts off, the user back, gone, the controller on.
local function stand(me: Object)
    local s = me.state :: StandInState
    if me.phase_init == 0 then
        POLTERGEIST.spawn(me, int.u32(me.damage + me.stamina * 0x10000), s.nearest)
        me.phase_init = 4
    end
    if s.busy then
        return
    end
    local user = me.related1 :: Object
    me:remove_parts_of(user)
    me:set_lifecycle("destroy")
    dimming.show_user(user)
    local c = s.controller
    if c and c.active then
        (c.state :: { busy: boolean }).busy = false
    end
end

-- 0x080C390C: drawn as the user in animation 12 where the user is, with its palette and (a player's) parts; the user
-- vanishes, its status visuals kept.
local function stand_in_init(me: Object)
    local user = me.related1
    if not user then
        error("Poltrgst's stand-in has no user (0x080C390C reads through a null pointer)")
    end
    local sprite = me.sprite
    sprite:load_like(user)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = STAND_IN_ANIM, 0xFF
    me.visible = true
    me.pos = user.pos
    me:set_panel_from_coordinates()
    -- sub_801002C: the user's navi palette.
    sprite.palette = user.sprite.palette
    -- 0x080C39A0: a player user's overlay (0x0800F0EC: its soul's put-on, NumberSoul's layer).
    if user.actor_type == "player" then
        me:add_overlay_of(user)
    end
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    dimming.hide_user_sparing(user)
    me:set_lifecycle("update")
    stand(me)
end

local STAND_IN = new.kind {
    id = "poltrgst/stand-in",
    pool = "actor",
    state = { busy = "bool", controller = "object", nearest = "bool" },
    -- 0x080C38E8.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            stand_in_init(me)
        elseif l == "update" then
            stand(me)
        else
            -- 0x080C39BE: the controller's flag cleared, freed.
            local c = (me.state :: StandInState).controller
            if c and c.active then
                (c.state :: { busy: boolean }).busy = false
            end
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- ---- The controller ------------------------------------------------------------------------------------------------

export type ControllerState = {
    -- The Atk+ bonus (+0x32): it adds to the stand-in's damage word.
    bonus: number,
    -- The stand-in is out (+0x0C, which it clears).
    busy: boolean,
    nearest: boolean,
}

-- 0x080E8C60: with an obstacle to take, the stand-in out (0x080C3A10: on the controller's panel, the element byte
-- its side, the damage word with the bonus, the user's side and facing, running while dimmed); when it is done (or
-- with none), done.
local function effect(me: Object)
    local s = me.state :: ControllerState
    if me.phase_init == 0 then
        if not common.any_taken() then
            dimming_chips.done(me)
            return
        end
        local user = me.related1 :: Object
        local o = battle.spawn(STAND_IN)
        if o then
            o.panel_x, o.panel_y = me.panel_x, me.panel_y
            o.element = me.alliance
            o:set_damage_word(dimming_chips.damage_plus(me, s.bonus))
            o.related1 = user
            local ss = o.state :: StandInState
            ss.controller, ss.nearest = me, s.nearest
            s.busy = true
            o.alliance, o.flip = user.alliance, user.flip
            o.run_while_dimmed = true
        end
        me.phase_init = 4
    end
    if not s.busy then
        dimming_chips.done(me)
    end
end

poltergeist.controller = new.kind {
    id = "poltrgst/controller",
    pool = "effect",
    state = { bonus = "u16", busy = "bool", nearest = "bool" },
    -- 0x080E8C1C.
    update = dimming_chips.phases { name = "Poltrgst", effect = effect },
}

-- 0x080E8CDA: the chip's `dimming` hook; `nearest`: its first parameter.
function poltergeist.hook(nearest: boolean): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(poltergeist.controller, user, spec)
        if o then
            local s = o.state :: ControllerState
            s.bonus, s.nearest = spec.bonus, nearest
        end
        return o
    end
end

return poltergeist
