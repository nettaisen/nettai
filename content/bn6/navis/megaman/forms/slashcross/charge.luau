--!strict
-- SlashCross's charged shot (`slashcross/charge`): the original's weapon
-- routine 0x12 (`sub_8011CD6`), and the charged slash both it and
-- SlashCross's A-charge (`megaman/slash-a-charge`,
-- ../../weapons/slash-a-charge) start: the original's action 0x41
-- (`sub_80EE34A`). The navi may first dash two panels forward (a step
-- sword's slash, as StepSwrd's), leaving an afterimage; it raises its
-- sword (the charged shot's own slash and MoonBld's swing the arm alone),
-- and on the tick its 21-tick count reaches 12 sends its slash's sword
-- wave (./sword_wave) from the panel in front (CrosSwrd's sends a second,
-- hidden one). DblDream's slashes twice. A dash ends back on the panel it
-- started from.
--
-- What the slash is comes with its `ChargedSlash` (./slashes), which the
-- action keeps in its state: the charged shot's own, or the one the
-- charged chip's sword names (`slash_charge.of_action`). (The original
-- keeps the charged chip's subtype in the attack's variant and its first
-- parameter, and reads each part of the slash from a table by it.)

local parts = require("../../../../lib/swords/parts")
local slashes = require("./slashes")
local sword_wave = require("./sword_wave")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- AIAttackVars+0x10: ticks into the slash, or of the end. (The slashes
    -- to go are the navi's `attack_count`.)
    timer: number,
    -- The slash (the original's attack variant), and whether the navi
    -- dashes in first (its first parameter).
    slash: ChargedSlash,
    dash: boolean,
}

local slash_charge = {}

-- The phases, on the attack's step.
local DASH, COUNT, SLASH, RETURN, FINISH = 0x0, 0x4, 0x8, 0xC, 0x10

-- The charged shot's attack: Sword (secondary element 0x80) and its hit
-- parameter.
local SWORD, HIT_PARAM = 0x80, 0x94

-- The panels a dash may end on (`sub_8015B00`, two ahead): with the
-- other side's body flag (0x10000) and none of 0xF800000.
local DASH_REQUIRE, DASH_FORBID = 0x10000, 0x0F800000

-- The dash's afterimage: its colour shader and lifetime.
local AFTERIMAGE_SHADER, AFTERIMAGE_TICKS = 0xFFE0, 0x14

-- The tick of the count the wave goes out on (the game means 17 for the
-- slashes without a blade, but compares the wrong register).
local WAVE_TICK = 12

-- The charged slash as `slash`, with a dash first or not: its action, for
-- `navi` to start. The Beast Out rush locks on in the slash's mode (the
-- original's table by the variant, `sub_80EAF26`).
function slash_charge.begin(navi: Object, slash: ChargedSlash, dash: boolean): Action
    local s = navi:action_state(slash_charge.action) :: State
    s.slash = slash
    s.dash = dash
    navi.rush_lockon = slash.lockon
    return slash_charge.action
end

-- The charged slash of a sword whose action is `action` (a slash of
-- lib/swords, or MoonBld's swing: what its spec names as `charged`, with
-- a step sword's dash). An action that names none slashes as the plain
-- sword (the original's row 0, what a chip without a subtype reads).
function slash_charge.of_action(navi: Object, action: Action): Action
    local spec = action.args :: { charged: ChargedSlash?, step: boolean? }?
    local slash = (spec and spec.charged) or slashes.sword
    return slash_charge.begin(navi, slash, spec ~= nil and spec.step == true)
end

-- `sub_8011CD6`: 60 damage and 20 more per point of buster Attack (up to
-- 5), a Sword attack: the charge's own slash, no dash.
local function setup(navi: Object): Action
    navi.attack_damage = weapon.charge_damage(navi, 0x3C, 0x14)
    navi.hit_param = HIT_PARAM
    navi.attack_element = SWORD
    navi.extra = 0
    return slash_charge.begin(navi, slashes.charge, false)
end

local function moving_or_sliding(me: Object): boolean
    return me:status("moving") or me:status("sliding")
end

-- `sub_80EE370`: for a dash with the navi still, dash two panels forward
-- if it can (leaving an afterimage where it stood); else no slash at all
-- (the animation 4, then back).
local function dash(me: Object, s: State)
    me.step = COUNT
    me.step_init = 0
    if not s.dash or moving_or_sliding(me) then
        return
    end
    me:set_status("moving", true)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me:reserve_panel(me.panel_x, me.panel_y)
    local x, y = me.panel_x + 2 * me.facing, me.panel_y
    if not field.check(x, y, DASH_REQUIRE, DASH_FORBID) then
        me:set_animation(4)
        me.step = RETURN
        me.step_init = 0
        return
    end
    -- A copy of the navi's sprite (Param1 0xFF), animation 0
    -- (`sub_800F2AA`'s nullsub leaves it), the navi's flip; a shadow kept
    -- on the ground (the lifetime word's bytes 2 and 3 set).
    battle.afterimage(me, me.pos, {
        anim = 0,
        flip = bit32.bxor(me.alliance, me.flip),
        lifetime = AFTERIMAGE_TICKS,
        color_shader = AFTERIMAGE_SHADER,
        shadow = "ground",
    })
    me.panel_x, me.panel_y = x, y
    me:set_coordinates_from_panel()
    me:update_collision_panels()
end

-- `sub_80EE3F0`: how many slashes.
local function count(me: Object, s: State)
    me.attack_count = if s.slash.again then 2 else 1
    me.marker = 0
    me.step = SLASH
    me.step_init = 0
end

-- `sub_80EE406`: raise the sword; send the wave on the 12th tick; after
-- 21 ticks slash again or go on.
local function slash(me: Object, s: State)
    if me.step_init == 0 then
        local slash = s.slash
        local blade = slash.blade
        me:set_animation(if blade then 5 else 0xC)
        me:refresh_form_overlay()
        me:open_counter_window()
        battle.play_sound(slash.sound)
        if blade then
            -- `sub_80EBB34`: the blade, in the first related slot.
            parts.hold(me, blade, parts.blade_palette(me))
        end
        me:set_status("using_action", true)
        s.timer = 0x15
        me.step_init = 4
    end
    if s.timer == WAVE_TICK then
        local slash = s.slash
        local x, y = me.panel_x + me.facing, me.panel_y
        local damage = int.u32(bit32.bor(me.attack_damage, bit32.lshift(me.hit_param, 16)) + me.extra)
        sword_wave.spawn(me, slash.wave, x, y, me.attack_element, damage)
        if slash.hidden_wave then
            sword_wave.spawn(me, slash.hidden_wave, x, y, me.attack_element, damage, true)
        elseif slash.again then
            -- The next slash is the second.
            s.slash = slash.again
        end
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    local left = me.attack_count - 1
    me.attack_count = left
    if left > 0 then
        -- Again from the entry. (The game returns through a mismatched
        -- `pop`, leaving r8 with the navi's AIData pointer.)
        me.related1 = nil
        me.overlay = nil
        me.step_init = 0
        return
    end
    me.step = if s.dash then RETURN else FINISH
    me.step_init = 0
end

-- `sub_80EE50C`: after the animation's last frame, back on the panel the
-- dash started from.
local function return_(me: Object)
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
    me:unreserve_panel(me.panel_x, me.panel_y)
    me:update_collision_panels()
    me:set_coordinates_from_panel()
    me.step = FINISH
    me.step_init = 0
end

-- `sub_80EE532`: 6 ticks, then idle.
local function finish(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("moving", false)
        s.timer = 5
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    me.related1 = nil
    me.overlay = nil
    me:exit_attack()
end

slash_charge.action = define.action {
    id = "slashcross/charge/action",
    state = { timer = "u16", slash = "record:charged-slash", dash = "bool" },
    -- `sub_80EE34A`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == DASH then
            dash(me, s)
        elseif step == COUNT then
            count(me, s)
        elseif step == SLASH then
            slash(me, s)
        elseif step == RETURN then
            return_(me)
        elseif step == FINISH then
            finish(me, s)
        else
            error(string.format("charged slash phase %#x reads past its phase table (sub_80EE34A)", step))
        end
    end,
}

slash_charge.weapon = define.weapon {
    id = "slashcross/charge",
    -- The charge table's row 0x12, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 80, 70, 60, 55, 50, 60, 60, 60 },
    setup = setup,
}

return slash_charge
