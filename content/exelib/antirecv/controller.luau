--!strict
-- AntiRecv's counterattack, as BN5 and BN6 share it (BN6's effect object
-- #0x2C, `sub_80E3728`; BN5's #0x2C): the dimming controller of a sprung
-- AntiRecv. Each game makes its own (`controller.make`: its kind, its panel
-- changer's and bubble's, the change's sounds). The ruleset
-- spawns it (`sub_80E37D2`; the role `kinds.anti_recovery`,
-- rules/roles.luau) in a heal's place when the healer's opponent has
-- AntiRecv armed: a recovery chip's heal (`sub_800E2FC`, which also starts
-- its dimming) or Roll's navi chip (`sub_80E192C`, whose action registers
-- it), with the healer as its first related object and the damage word: the
-- heal's amount, or three times Roll's damage, with hit parameter 0x1E.
-- The usual dimming phases, AntiRecv's telop, and the effect: the healer
-- loses the damage, two bubbles rise either side of the controller's panel,
-- and the panel changer poisons the healer's panel; once it is done, the
-- undim. See docs/engine/chips.md §3.6.7.

local dimming_chips = require("../dimming")
local panel_changer = require("@exelib/panel-changer/panel_changer")
local rising_bubble = require("@exelib/rising-bubble/rising_bubble")

export type State = {
    -- The panel changer runs (the original's Param2, which the changer
    -- clears).
    busy: boolean,
}

local controller = {}

-- How long the effect's first tick sets the timer for (nothing counts it).
local EFFECT_TICKS = 0x3C
-- The bubbles: 16 pixels either side of the panel's center, palette 1.
local BUBBLE_DX, BUBBLE_PALETTE = 0x10_0000, 1
-- A game's counterattack: its kind's key, its panel changer's and bubble's
-- kinds, the sounds of the change (its tick, and the poison), and whether
-- it takes the healer's mood too, by its damage word's high half (BN5's,
-- 0x080E39DC: 0x08012820, to 1 at least; BN6's doesn't).
export type Spec = {
    id: string,
    changer: Kind,
    bubble: Kind,
    tick_sound: Sound,
    poison_sound: Sound,
    takes_mood: boolean?,
}

-- `sub_80E376C`: the damage, the bubbles and the poison (`change`); then
-- wait until the changer is done (it was handed Param2's address, and
-- clears it), and undim.
local function effect(me: Object, spec: Spec, POISON_OWN_PANEL: PanelChange)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.timer = EFFECT_TICKS
        -- object_subtractHP on the healer (down to 0, by its side's rules:
        -- BN5's may hold a dark MegaMan at 1 HP).
        local healer = me.related1
        if not healer then
            error("AntiRecv's counterattack has no healer (object_subtractHP writes through a null pointer)")
        end
        healer:subtract_hp(me.damage)
        if spec.takes_mood then
            battle.lose_mood(healer.alliance, me.stamina)
        end
        local x, y = field.center(me.panel_x, me.panel_y)
        rising_bubble.spawn(spec.bubble, Vec3.new(x + BUBBLE_DX, y, 0), BUBBLE_PALETTE)
        rising_bubble.spawn(spec.bubble, Vec3.new(x - BUBBLE_DX, y, 0), BUBBLE_PALETTE)
        panel_changer.spawn(spec.changer, me, me.panel_x, me.panel_y, POISON_OWN_PANEL, me)
        s.busy = true
        me.phase_init = 4
    end
    if not s.busy then
        dimming_chips.done(me)
    end
end

-- A game's counterattack.
function controller.make(spec: Spec): { kind: Kind }
    -- The panel changer's change (`byte_80E272C` row 6): the own panel turns
    -- to poison, in 64 ticks, flickering.
    local change = panel_changer.change {
        panel = "poison",
        panels = "own",
        flicker = true,
        ticks = 0x40,
        sides = { [0] = { require = 0x0, forbid = 0x8020 }, [1] = { require = 0x20, forbid = 0x8000 } },
        tick_sound = spec.tick_sound,
        change_sound = spec.poison_sound,
    }

    -- `sub_80E3728` (states) and `sub_80E3748` (`off_80E375C`, by action).
    local kind = define.kind {
        id = spec.id,
        pool = "effect",
        state = { busy = "bool" },
        update = dimming_chips.phases {
            name = "AntiRecv's counterattack",
            effect = function(me: Object)
                effect(me, spec, change)
            end,
        },
    }

    return { kind = kind }
end

return controller
