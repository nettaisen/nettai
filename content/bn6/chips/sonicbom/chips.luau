--!strict
-- SonicBom, SprSonic, Curse and Punisher: the navi swings its sword and
-- throws a sonic boom (lib/swords/sonic_boom.luau) from the panel ahead on
-- the swing's tenth tick (the original's action 0x55, `sub_80EF970`;
-- docs/engine/shot-chips.md §12). SprSonic's goes through what it hits.
--
-- Their records stay the pack's for now (VarSwrd and NeoVari start
-- SonicBom and SprSonic by number, keeping the sword's damage; Curse is
-- VDoll's curse's telop): these are their actions, which registration by
-- number runs (docs/design/content-model-v2.md §5.1, "As built").

local parts = require("../../lib/swords/parts")
local sonic_boom = require("../../lib/swords/sonic_boom")

local sonicbom = {}

export type State = {
    -- The swing's countdown (AIAttackVars+0x10).
    timer: number,
}

sonicbom.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local COUNT, SLASH, RECOVER = 0x0, 0x4, 0x8

local ANIM_SWING = 5
-- The swing's ticks (less one), the count the boom leaves on, and the
-- recovery's.
local SWING_TICKS, THROW_TICK, RECOVER_TICKS = 0x15, 0xC, 5
local SLASHES = 1

local function action(id: string, boom: SonicBoom): Action
    -- `sub_80EF990`: one slash (the shots word, AIAttackVars+0x12).
    local function count(me: Object)
        me.recovery = SLASHES
        me.step, me.step_init = SLASH, 0
    end

    -- `sub_80EF9A2`: swing the sword; the boom on the count's 0xC; at the
    -- swing's end the next slash, or the recovery.
    local function slash(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_SWING)
            me:open_counter_window()
            battle.play_sound(parts.sounds.swing)
            parts.hold(me, parts.blades.sword, 0)
            me:set_status("using_action", true)
            s.timer = SWING_TICKS
            me.step_init = 4
        end
        if s.timer == THROW_TICK then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            sonic_boom.throw(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage, boom)
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        local left = me.recovery - 1
        me.recovery = left
        if left > 0 then
            parts.drop(me)
            me.step_init = 0
        else
            me.step, me.step_init = RECOVER, 0
        end
    end

    -- `sub_80EFA30`: 6 ticks, then the sword goes and back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            parts.drop(me)
            me:exit_attack()
        end
    end

    return define.action {
        id = id,
        state = sonicbom.STATE,
        args = boom,
        -- `sub_80EF970`.
        update = function(me: Object, s: State)
            local step = me.step
            if step == COUNT then
                count(me)
            elseif step == SLASH then
                slash(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("SonicBom phase %#x reads past off_80EF984", step))
            end
        end,
    }
end

-- The booms: SonicBom's staggers (hit modifier 3), SprSonic's goes
-- through (modifier 1), Curse's and Punisher's are plain.
sonicbom.actions = {
    sonicbom = action("sonicbom/action", { hit_mod = 3, pierces = false }),
    sprsonic = action("sprsonic/action", { hit_mod = 1, pierces = true }),
    curse = action("curse/action", { hit_mod = 0, pierces = false }),
    punisher = action("punisher/action", { hit_mod = 0, pierces = false }),
}

return sonicbom
