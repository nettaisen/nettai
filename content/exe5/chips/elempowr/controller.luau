--!strict
-- ElemPowr's controller (effect object #0x7F, 0x080E9B68): EXE5's own, the panel chips' (@exelib/panel_chips, EXE6's
-- `sub_80E2AE8`) reworked. The usual dimming phases; its effect (0x080E9BAC), when the user stands on lava, sea, ice,
-- magnet or grass, adds 10 for each panel of that type on the field (at most 9) to the side's Atk+ bonus, and has a
-- panel changer (objects/panel_changer: EXE5's rows 15 to 19, 0x080E2A78) turn every such panel back to normal over 30
-- ticks, then waits for it; on any other panel it is done at once.

local dimming_chips = require("@exelib/dimming")
local panel_changer = require("../../objects/panel_changer")

local controller = {}

export type State = {
    -- The changer runs (+0x0C, which it clears).
    busy: boolean,
}

-- The side's bonuses (`battle.add_special_bonus`'s index): the Atk+ bonus (the side state's +0x36).
local ATTACK_BONUS = 0
local PER_PANEL, MAX_PANELS = 10, 9
-- The flag bit each type's panels have (EXE5's panel types' words, 0x0800BCF0), what any panel the change touches
-- mustn't have (0x8000), and the change by type.
local NOT = 0x8000
local function back_to_normal(bit: number): PanelChange
    local side = { require = bit, forbid = NOT }
    return panel_changer.change {
        panel = "normal",
        panels = "all",
        flicker = true,
        ticks = 30,
        sides = { [0] = side, [1] = side },
        tick_sound = panel_changer.TICK_SOUND,
        change_sound = panel_changer.CHANGE_SOUND,
    }
end
local CHANGES: { [string]: PanelChange } = {
    lava = back_to_normal(0x1000),
    sea = back_to_normal(0x2_0000),
    ice = back_to_normal(0x800),
    magnet = back_to_normal(0x200),
    grass = back_to_normal(0x400),
}

-- `object_dead_getPanelsTypeCount`: the field's panels of `kind`.
local function count(kind: PanelType): number
    local n = 0
    for y = 1, 3 do
        for x = 1, 6 do
            local p = field.panel(x, y)
            if p and p.kind == kind then
                n += 1
            end
        end
    end
    return n
end

-- 0x080E9BAC.
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        local p = field.panel(me.panel_x, me.panel_y)
        local kind = if p then p.kind else "missing"
        local change = CHANGES[kind]
        if not change then
            dimming_chips.done(me)
            return
        end
        panel_changer.spawn(me, me.panel_x, me.panel_y, change, me)
        s.busy = true
        local bonus = PER_PANEL * math.min(count(kind), MAX_PANELS)
        battle.add_special_bonus(me.alliance, ATTACK_BONUS, bonus)
        me.phase_init = 4
    end
    if not s.busy then
        dimming_chips.done(me)
    end
end

controller.kind = new.kind {
    id = "elempowr/controller",
    pool = "effect",
    state = { busy = "bool" },
    -- 0x080E9B68.
    update = dimming_chips.phases { name = "ElemPowr", effect = effect },
}

-- 0x080E9C40: the chip's `dimming` hook (the spawner copies the user's side, not its facing).
function controller.hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(controller.kind, user, spec, true)
end

return controller
