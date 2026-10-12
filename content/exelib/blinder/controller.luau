--!strict
-- Blinder's dimming controller (effect object #0x23: EXE6's `sub_80E2E34`, which no EXE6 chip uses; EXE5's
-- 0x080E3050; EXE4's 0x080E48EC, the same code), made for a game's look (`controller.make`): the usual dimming phases
-- (@exelib/dimming), with an effect where the user flashes white for 10 ticks, then a screen flash, the sound and the
-- blinding hit over the other side's area (no damage, no spark: `byte_80E2F18`'s region words by side, resolving
-- while dimmed), and 60 more ticks of the user flashing. Its spawn (`sub_80E2F24`) takes the user's side alone.
--
-- The games part in the look: the kind's id, the sound, and the blinding hit's self type, target and status.

local dimming_chips = require("../dimming")
local regions = require("../regions")

local controller = {}

-- The phases of its effect.
local FLASH, BLIND = 0x0, 0x4

-- A game's Blinder: the chip's `dimming` hook.
function controller.make(look: BlinderLook): (user: Object, spec: DimmingChipSpec) -> Object?
    -- The user flashes white this tick.
    local function whiten(me: Object)
        local user = me.related1
        if user then
            user.sprite.white = true
        end
    end

    -- `sub_80E2E94`: the user flashes for 10 ticks.
    local function flash(me: Object)
        if me.phase_init == 0 then
            me.timer = 10
            me.phase_init = 4
        end
        whiten(me)
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me.phase, me.phase_init = BLIND, 0
        end
    end

    -- `sub_80E2EBC`: a 60-tick screen flash, the sound and the blinding hit over the other side's area; 60 ticks of
    -- the user flashing, then undim.
    local function blind(me: Object)
        if me.phase_init == 0 then
            me.timer = 60
            battle.palette_flash(0, 60, true, false)
            battle.play_sound(look.sound)
            local h = battle.hitbox(me, {
                panel_x = me.panel_x,
                panel_y = me.panel_y,
                element = me.element,
                z = 0,
                region = regions.area[1 - me.alliance],
                hit_effect = nil,
                target = look.hits,
                self_type = look.is,
                damage = 0,
                stamina = 0,
                hit_mod = 0,
                status = look.status,
            })
            -- (`sub_80C53A6`: it resolves while dimmed.)
            if h then
                h.run_while_dimmed = true
            end
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            dimming_chips.done(me)
            return
        end
        whiten(me)
    end

    -- `sub_80E2E78`.
    local function effect(me: Object)
        local phase = me.phase
        if phase == FLASH then
            flash(me)
        elseif phase == BLIND then
            blind(me)
        else
            error(string.format("Blinder's phase %#x reads past off_80E2E8C", phase))
        end
    end

    local kind = new.kind {
        id = look.id,
        pool = "effect",
        -- `sub_80E2E34`.
        update = dimming_chips.phases { name = "Blinder", effect = effect },
    }

    return function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(kind, user, spec, true)
    end
end

return controller
