--!strict
-- AntiRecv's counterattack (effect object #0x2C, `sub_80E3728`): the
-- dimming controller of a sprung AntiRecv. The ruleset spawns it
-- (`sub_80E37D2`) in a heal's place when the healer's opponent has AntiRecv
-- armed: a recovery chip's heal (`sub_800E2FC`, which also starts its
-- dimming) or Roll's navi chip (`sub_80E192C`, whose action registers it),
-- with the healer as its first related object and the damage word: the
-- heal's amount, or three times Roll's damage, with hit parameter 0x1E.
-- The usual dimming phases, AntiRecv's telop (chip 0xBD), and the effect:
-- the healer loses the damage, two bubbles rise either side of the
-- controller's panel, and the panel changer (kind 6) poisons the healer's
-- panel; once it is done, the undim. See docs/engine/chips.md §3.6.7.

local panel_changer = require("../panel-changer/panel_changer")
local rising_bubble = require("../rising-bubble/rising_bubble")

local anti_recovery = {}

-- How long the effect's first tick sets the timer for (nothing counts it).
local EFFECT_TICKS = 0x3C
-- The bubbles: 16 pixels either side of the panel's center, palette 1.
local BUBBLE_DX, BUBBLE_PALETTE = 0x10_0000, 1
-- The panel changer's kind: the own panel turns to poison.
local POISON_OWN_PANEL = 6

-- `sub_80E376C`: the damage, the bubbles and the poison; then wait until
-- the changer clears the controller's parameters (it was handed Param2's
-- address), and undim.
local function effect(me: Object)
    if me.phase_init == 0 then
        me.timer = EFFECT_TICKS
        -- object_subtractHP on the healer (down to 0).
        local healer = me.related1
        if not healer then
            error("AntiRecv's counterattack has no healer (object_subtractHP writes through a null pointer)")
        end
        healer.hp = math.max(healer.hp - me.damage, 0)
        local x, y = field.center(me.panel_x, me.panel_y)
        rising_bubble.spawn(Vec3.new(x + BUBBLE_DX, y, 0), BUBBLE_PALETTE)
        rising_bubble.spawn(Vec3.new(x - BUBBLE_DX, y, 0), BUBBLE_PALETTE)
        panel_changer.spawn(me, me.panel_x, me.panel_y, POISON_OWN_PANEL, me)
        me:set_param(2, 1)
        me.phase_init = 4
    end
    if me:param(2) == 0 then
        me:set_action(0xC)
    end
end

-- `sub_80E3728` (states) and `sub_80E3748` (`off_80E375C`, by action).
function anti_recovery.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        dimming.begin(me)
    elseif l == "update" then
        local a = me.action
        if a == 0 then
            dimming.dim_screen(me)
        elseif a == 4 then
            dimming.show_telop(me)
        elseif a == 8 then
            effect(me)
        elseif a == 0xC then
            dimming.undim_screen(me)
        else
            error(string.format("AntiRecv's counterattack's action %#x reads past off_80E375C", a))
        end
    else
        dimming.finish(me)
    end
end

return anti_recovery
