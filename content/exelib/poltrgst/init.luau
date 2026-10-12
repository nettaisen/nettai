--!strict
-- Poltrgst's controller and stand-in, as EXE5 and EXE4 share them (the dimming handler's controller, effect object
-- #0x72: EXE5's 0x080E8C1C, EXE4's 0x080EA05C; the stand-in, actor object #0x58: EXE5's 0x080C38E8, EXE4's
-- 0x080CC624): the usual dimming phases, whose effect (EXE5's 0x080E8C60, EXE4's 0x080EA0A0), when the field has an
-- obstacle the poltergeist takes, sends out a stand-in of the user (drawn as the user, posed, while the user vanishes,
-- its status visuals kept), which raises the game's poltergeist (@exelib/poltergeist) and waits on it; then the
-- stand-in gives the user back and goes, and the controller is done. With no obstacle to take it is done at once.
--
-- Each game makes its own (`poltrgst.make`): its kinds' keys, its poltergeist, the stand-in's pose, and where the
-- games' code differs (EXE5's additions and EXE4's one test, the spec's flags).

local common = require("@exelib/poltergeist")
local dimming_chips = require("@exelib/dimming")

local poltrgst = {}

export type Spec = {
    -- The controller's and the stand-in's kinds' keys.
    controller_id: string,
    stand_in_id: string,
    -- The game's poltergeist (@exelib/poltergeist's `make`).
    poltergeist: { kind: Kind, spawn: (holder: Object, damage: number, nearest: boolean) -> Object? },
    -- The stand-in's animation (EXE5's 12, EXE4's 10).
    anim: number,
    -- EXE4's (0x080CC64C): only MegaMan has a stand-in (the navi's +0x0F 0); another's goes at once, its controller
    -- left waiting.
    megaman_only: boolean?,
    -- EXE5's (0x080C39A0): a player user's overlay on the stand-in, its parts off as it goes.
    wears_overlay: boolean?,
    -- EXE5's: the chip's Atk+ bonus on the stand-in's damage word (the controller's +0x32); EXE4's is the chip's
    -- damage word alone.
    adds_bonus: boolean?,
    -- EXE5's (0x080C39BE): the stand-in's destroy state clears its controller's flag again; EXE4's is freed.
    destroy_tells_controller: boolean?,
    -- EXE5's: the controller and the stand-in take the user's facing with its side; EXE4's spawners copy the side
    -- alone (0x080EA118, 0x080CC6FE).
    takes_facing: boolean?,
}

-- A game's Poltrgst: its controller's kind, and the chip's `dimming` hook (EXE5's 0x080E8CDA, EXE4's 0x080EA118);
-- `nearest`: the chip's first parameter, the poltergeist's.
export type Poltrgst = {
    controller: Kind,
    hook: (nearest: boolean) -> (user: Object, spec: DimmingChipSpec) -> Object?,
}

type StandInState = {
    -- The poltergeist runs (+0x0C, which it clears).
    busy: boolean,
    -- Its controller, waiting on it (its +0x60 pointer to the controller's flag).
    controller: Object?,
    nearest: boolean,
}

type ControllerState = {
    -- The Atk+ bonus (+0x32).
    bonus: number,
    -- The stand-in is out (+0x0C, which it clears).
    busy: boolean,
    nearest: boolean,
}

function poltrgst.make(spec: Spec): Poltrgst
    -- The controller isn't waiting on its stand-in.
    local function tell(me: Object)
        local c = (me.state :: StandInState).controller
        if c and c.active then
            (c.state :: { busy: boolean }).busy = false
        end
    end

    -- EXE5's 0x080C39CA, EXE4's 0x080CC6C6: the poltergeist out; when it is done, (the parts off,) gone, the user back,
    -- the controller on.
    local function stand(me: Object)
        local s = me.state :: StandInState
        if me.phase_init == 0 then
            spec.poltergeist.spawn(me, int.u32(me.damage + me.stamina * 0x10000), s.nearest)
            me.phase_init = 4
        end
        if s.busy then
            return
        end
        local user = me.related1 :: Object
        if spec.wears_overlay then
            me:remove_parts_of(user)
        end
        me:set_lifecycle("destroy")
        dimming.show_user(user)
        tell(me)
    end

    -- EXE5's 0x080C390C, EXE4's 0x080CC648: drawn as the user (its sprite in its form), with its shadow, in the
    -- stand-in's animation where the user is, with its palette (and a player's overlay); the user vanishes, its
    -- status visuals kept. False: it went at once.
    local function stand_in_init(me: Object): boolean
        local user = me.related1
        if not user then
            error("Poltrgst's stand-in has no user (its init reads through a null pointer)")
        end
        if spec.megaman_only and user.actor_type ~= "player" then
            me:free()
            return false
        end
        local sprite = me.sprite
        sprite:load_like(user)
        sprite.shadow = "ground"
        me.anim, me.anim_loaded = spec.anim, 0xFF
        me.visible = true
        me.pos = user.pos
        me:set_panel_from_coordinates()
        -- `sub_801002C` (EXE4's 0x0800BFFE): the user's navi palette.
        sprite.palette = user.sprite.palette
        if spec.wears_overlay and user.actor_type == "player" then
            me:add_overlay_of(user)
        end
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        dimming.hide_user_sparing(user)
        me:set_lifecycle("update")
        stand(me)
        return true
    end

    local stand_in = new.kind {
        id = spec.stand_in_id,
        pool = "actor",
        state = { busy = "bool", controller = "object", nearest = "bool" },
        -- EXE5's 0x080C38E8, EXE4's 0x080CC624.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                if not stand_in_init(me) then
                    return
                end
            elseif l == "update" then
                stand(me)
            else
                if spec.destroy_tells_controller then
                    tell(me)
                end
                me:free()
                return
            end
            me:update_sprite()
        end,
    }

    -- EXE5's 0x080E8C60, EXE4's 0x080EA0A0: with an obstacle to take, the stand-in out (EXE5's 0x080C3A10, EXE4's
    -- 0x080CC6FE: on the controller's panel, the element byte its side, the damage word (with EXE5's bonus), the
    -- user's side (and EXE5's facing), running while dimmed); when it is done (or with none), done.
    local function effect(me: Object)
        local s = me.state :: ControllerState
        if me.phase_init == 0 then
            if not common.any_taken() then
                dimming_chips.done(me)
                return
            end
            local user = me.related1 :: Object
            local o = battle.spawn(stand_in)
            if o then
                o.panel_x, o.panel_y = me.panel_x, me.panel_y
                o.element = me.alliance
                o:set_damage_word(dimming_chips.damage_plus(me, if spec.adds_bonus then s.bonus else 0))
                o.related1 = user
                local ss = o.state :: StandInState
                ss.controller, ss.nearest = me, s.nearest
                s.busy = true
                o.alliance = user.alliance
                if spec.takes_facing then
                    o.flip = user.flip
                end
                o.run_while_dimmed = true
            end
            me.phase_init = 4
        end
        if not s.busy then
            dimming_chips.done(me)
        end
    end

    local controller = new.kind {
        id = spec.controller_id,
        pool = "effect",
        state = { bonus = "u16", busy = "bool", nearest = "bool" },
        -- EXE5's 0x080E8C1C, EXE4's 0x080EA05C.
        update = dimming_chips.phases { name = "Poltrgst", effect = effect },
    }

    local function hook(nearest: boolean): (user: Object, spec: DimmingChipSpec) -> Object?
        return function(user: Object, chip: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(controller, user, chip, not spec.takes_facing)
            if o then
                local s = o.state :: ControllerState
                s.bonus, s.nearest = chip.bonus, nearest
            end
            return o
        end
    end

    return { controller = controller, hook = hook }
end

return poltrgst
