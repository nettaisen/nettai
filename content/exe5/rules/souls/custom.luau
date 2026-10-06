--!strict
-- Soul Unison on EXE5's custom screen (docs/design/rules-in-luau.md §4.4, docs/design/exe5-map.md §15.8): the soul
-- button in the special slot (0x08023C54, its offer 0x08024B28, A on it 0x08024972), the soul's choice as the window
-- `soul_unison` (the screen's state 9, 0x080232D0), the soul given at OK (0x08024FF6) and the round's souls used
-- (0x02034E10). Its state is the rules' (rules/init.luau). What a soul adds to the screen (its slots, 0x08023CF8, and
-- its part of the screen's routines) is the soul's own: its form's `custom` (a `SoulCustom`, in the soul's folder:
-- navis/megaman/forms/<soul>), which this module's end gathers for the rules from the souls of the navi that has
-- them, and asks while the side's navi is in the soul.

local megaman = require("../../navis/megaman").megaman
local operation_battle = require("../../lib/operation_battle")

local custom_souls = {}

-- The rules' state fields the screen keeps (rules/init.luau declares them).
type State = {
    -- 0x02034E10: the souls given this round, a bit a soul for its Soul Unison and another for its Chaos Unison
    -- (`used_bit`; the round's first screen forgets them).
    souls_used: number,
    -- The button's offer (slot 11's +5 and +6): the soul (none: no offer), and whether a dark chip makes it Chaos
    -- Unison. Once the soul is given, the soul given.
    offer: FormDef?,
    offer_chaos: boolean,
    -- The window's step (0x080232F0's offsets 0 to 0x18) and its count (+0x40).
    unite_step: number,
    unite_count: number,
    -- The turn's arm chip (the transform record's +6; none: its 0xFFFF): each screen's open clears it, a soul that
    -- takes one fills it at OK (ColonelSoul's Arm Change), and the soul change copies it into the navi's weapon chip
    -- (./change).
    arm_chip: Chip?,
}

-- What the player's save brings (rules/init.luau declares it): the souls it has (the original's flags, 0x08024BF0's, one a
-- soul; none for a save without Soul Unison, event flag 0: no button) and Chaos Unison (event flag 0x236).
type Setup = {
    souls: { FormDef? },
    chaos_unison: boolean,
}

-- The sequence's fades' speed (`SetScreenFade(m, 32)`).
local FADE_SPEED = 32
-- A Soul Unison's turns (+ the NaviCust's and the cards' bonus, at most 9; under 0, 1), a Chaos Unison's.
local TURNS, MAX_TURNS, CHAOS_TURNS = 3, 9, 1

-- What the souls keep of a screen, the rules' state's `screen` (a record): each soul's `custom.state`, its fields by
-- their declared types (this module's end gathers them with the souls' buttons and windows).
local screen: StateSchema = {}

-- The whole numbers a state field may be declared as.
local INTEGERS: { [string]: boolean } = { u8 = true, u16 = true, u32 = true, i8 = true, i16 = true, i32 = true }

-- What a fresh state holds in a field declared as `kind` (a state starts all zero, like the game's scratch): 0 for a
-- number, false for a boolean, the first of a list of variants, and for anything else (a definition, an asset, an
-- object) none.
local function fresh(kind: SchemaField): any
    if type(kind) == "table" then
        return kind[1]
    elseif kind == "bool" then
        return false
    elseif kind == "vec3" then
        return Vec3.new(0, 0, 0)
    elseif INTEGERS[kind] then
        return 0
    end
    return nil
end

-- 0x08022CA2: the screen's record is zeroed as it opens. What every soul keeps of a screen (`screen`) is back as a
-- fresh state has it, from the fields' own declarations, whatever soul the side's navi is in: a soul's fields don't
-- outlast the screen they were set on.
local function forget_screen(s: any)
    local kept = s.screen
    for name, kind in screen do
        local element, count
        if type(kind) == "string" then
            element, count = string.match(kind, "^(.+)%[(%d+)%]$")
        end
        if element ~= nil then
            -- (An array: each of its elements.)
            for i = 1, tonumber(count) :: number do
                kept[name][i] = fresh(element)
            end
        else
            kept[name] = fresh(kind)
        end
    end
end

-- The side's navi's souls (MegaMan's: the forms with a `soul`).
local function souls_of(side: number): { FormDef }
    local forms = battle.navi(side).navi.forms
    if forms == nil or forms.souls == nil then
        return {}
    end
    return forms.souls
end

-- The soul a chip of `family` gives up for (the first of the navi's souls for that family).
local function soul_for(side: number, family: string): FormDef?
    for _, form in souls_of(side) do
        local soul = form.soul
        if soul ~= nil and soul.family == family then
            return form
        end
    end
    return nil
end

-- The setup's souls' places (./init's `form[16]`).
local SOUL_PLACES = 16

-- The save has `soul` (its flag: it is one of the setup's souls).
local function owned(soul: FormDef): boolean
    local setup = rules.setup() :: Setup
    for i = 1, SOUL_PLACES do
        if setup.souls[i] == soul then
            return true
        end
    end
    return false
end

-- The save has souls (Soul Unison: the button).
local function has_souls(): boolean
    local setup = rules.setup() :: Setup
    for i = 1, SOUL_PLACES do
        if setup.souls[i] ~= nil then
            return true
        end
    end
    return false
end

-- A soul's bit in the round's souls given: by its place among the side's navi's souls for its Soul Unison, 16
-- places on for its Chaos Unison.
local function used_bit(side: number, soul: FormDef, chaos: boolean): number
    local place = table.find(souls_of(side), soul)
    if place == nil then
        error(string.format("%s is none of the side's navi's souls", soul.id :: string))
    end
    return bit32.lshift(1, if chaos then 16 + place else place)
end

-- 0x08024B28: the soul the last pick offers: a chip (not the Regular chip, not a link navi's, with no capsule mixed
-- into it: the slot's +5) of a family one of the navi's souls is for, a soul the save has (a dark chip's Chaos
-- Unison needs the save's Chaos Unison), not given yet this round (Soul Unison and Chaos Unison apart). None:
-- (nil, false).
local function offer(side: number): (FormDef?, boolean)
    local last = custom.last_pick(side)
    if last == nil or last.regular or last.navi_chip or last.attached then
        return nil, false
    end
    local chip = last.chip
    local chaos = table.find(chip.flags or {}, "dark") ~= nil
    if chaos and not (rules.setup() :: Setup).chaos_unison then
        return nil, false
    end
    local family = chip.family
    local form = if family ~= nil then soul_for(side, family) else nil
    if form == nil or form.soul == nil then
        return nil, false
    end
    if not owned(form) then
        return nil, false
    end
    if bit32.btest((rules.state() :: State).souls_used, used_bit(side, form, chaos)) then
        return nil, false
    end
    return form, chaos
end

-- What the soul the side's navi is in adds to the screen (its form's `custom`), if the navi is in one of its souls
-- (the screen's +0x10: the side's navi is MegaMan; the soul: NaviStats +0x2C) and the soul adds anything.
local function united(side: number): SoulCustom?
    local form = battle.navi(side).form
    if form.soul == nil or table.find(souls_of(side), form) == nil then
        return nil
    end
    return form.custom
end

-- The same, for the screen's slots (0x08023CF8): they are by the soul the emotion routine leaves in r1 (0x08023C3C:
-- NaviStats +0x2C), which in battle mode 1 is the stats' address instead (0x080127C0): no soul's slots there.
local function slots_of(side: number): SoulCustom?
    return if battle.mode() ~= 1 then united(side) else nil
end

-- The soul button (0x08023C54), in the special slot: it offers a form (the state's `offer` and `offer_chaos`).
custom_souls.button = {
    slot = 11,
    view = "form_offer",
    -- MegaMan with souls, outside the operation battle, with the save's souls (its Soul Unison), unless his emotion is
    -- worried or dark (0x08012740: in a soul or angry he may; at mood 0 he is dark, below 65 worried, at 0xFF Full
    -- Synchro).
    shown = function(side: number): boolean
        local stats = battle.navi(side)
        local in_soul = stats.form.soul ~= nil
        local mood = stats.mood
        local hidden = not in_soul
            and custom.player(side).emotion ~= "angry"
            and (mood == 0 or (mood ~= 0xFF and mood < 65))
        return #souls_of(side) > 0 and has_souls() and not operation_battle()
            and not hidden
    end,
    -- 0x08024B28: unavailable until the last pick offers a soul, which the button keeps.
    state = function(side: number): "selectable" | "unavailable"
        local s = rules.state() :: State
        local form, chaos = offer(side)
        s.offer, s.offer_chaos = form, chaos
        return if form == nil then "unavailable" else "selectable"
    end,
    -- 0x08024972: a soul on offer starts its sequence.
    pressed = function(side: number)
        if custom.cursor_state(side) ~= "selectable" then
            custom.refuse(side)
            return
        end
        custom.play(side, "pick")
        local s = rules.state() :: State
        s.unite_step, s.unite_count = 0, 0
        custom.open_window(side, "soul_unison")
    end,
} :: ButtonSpec

-- The screen's state 9 (0x080232D0): the soul's icon flies to the column (16 ticks, then 8 sliding up), the screen
-- flashes (fades 0x34 and 0x30) and whitens (fade 4); white, the soul takes the place of the chip given up for it,
-- first in the selection, the button is picked, and the screen clears (fade 0); then the picking goes on. Each step
-- after the flash waits for the fade before it.
-- What the state draws after its step (0x080232DC): the emblem, the Regular chip's frame, and the chip the Arm
-- Change button holds (0x080254F4).
local function drawn(side: number)
    custom.draw_emblem(side, 0)
    custom.draw_regular(side)
    custom.draw_held(side)
end

custom_souls.window = {
    view = "offer_flight",
    update = function(side: number): boolean
        local s = rules.state() :: State
        local fading = custom.fading(side)
        local step, count = s.unite_step, s.unite_count
        if step == 0 then
            -- 0x0802330C
            step, count = 4, 0
        elseif step == 4 then
            -- 0x08023360
            count += 1
            if count >= 16 then
                step, count = 8, 0
            end
        elseif step == 8 then
            -- 0x0802337A
            count += 1
            if count >= 8 then
                custom.fade(side, "flash", FADE_SPEED)
                custom.play(side, "program_advance_part")
                step, count = 12, 0
            end
        elseif step == 12 then
            -- 0x080233A8
            if not fading then
                custom.fade(side, "flash_back", FADE_SPEED)
                step, count = 16, 0
            end
        elseif step == 16 then
            -- 0x080233C4
            if not fading then
                custom.fade(side, "end_to_white", FADE_SPEED)
                step, count = 20, 0
            end
        elseif step == 20 then
            -- 0x080233E0
            if not fading then
                custom.trade_last_pick(side, "soul")
                custom.set_button_state(side, "soul", "selected")
                custom.update_availability(side)
                custom.fade(side, "intro_from_white", FADE_SPEED)
                custom.play(side, "program_advance")
                step, count = 24, 0
            end
        elseif step == 24 then
            -- 0x08023452
            if not fading then
                drawn(side)
                return false
            end
        else
            error(string.format("the soul's choice's step %#x reads past its table (0x080232F0)", step))
        end
        s.unite_step, s.unite_count = step, count
        -- (The count is the screen's frame counter, +0x40: the cursor blinks on from where it leaves it.)
        custom.set_frame(side, count)
        drawn(side)
        return true
    end,
} :: WindowSpec

-- The screen opens: the round's first forgets the round's souls, and there is no arm chip for the turn (0x080123D4:
-- the transform record's +6 0xFFFF).
function custom_souls.open(_side: number)
    local s = rules.state() :: State
    if battle.turn() == 1 then
        s.souls_used = 0
    end
    s.arm_chip = nil
end

-- The screen's hand (0x08025BE4): the soul's, if it says (NumberSoul's ten).
function custom_souls.hand_size(side: number): number?
    local part = united(side)
    if part ~= nil and part.hand_size ~= nil then
        return part.hand_size(side, rules.state())
    end
    return nil
end

-- The screen deals (0x08022CFC to 0x08023BF4: its hand, then its slots): what the souls keep of a screen starts
-- fresh, every soul's (`forget_screen`: out of MeddySoul there are no capsules), and then the soul the navi is in
-- lays its slots out (MeddySoul draws its capsules).
function custom_souls.deal(side: number)
    local s = rules.state()
    forget_screen(s)
    local part = slots_of(side)
    if part ~= nil and part.deal ~= nil then
        part.deal(side, s)
    end
end

-- 0x08024FF6: OK with the soul picked: the turn's form is the soul, for 3 turns and the save's bonus (at most 9;
-- under 0, 1), a Chaos Unison for 1; the soul is given this round.
function custom_souls.confirmed(side: number)
    local s = rules.state() :: State
    -- (0x08024EC0: the soul MegaMan is in first: ColonelSoul's arm chip.)
    local part = united(side)
    if part ~= nil and part.confirmed ~= nil then
        part.confirmed(side, s)
    end
    if not custom.button_picked(side, "soul") then
        return
    end
    local turns = TURNS + battle.navi(side).soul_turn_bonus
    if s.offer_chaos then
        turns = CHAOS_TURNS
    elseif turns > MAX_TURNS then
        turns = MAX_TURNS
    elseif turns < 0 then
        turns = 1
    end
    local soul = s.offer :: FormDef
    custom.set_form(side, "soul", soul, turns, s.offer_chaos)
    s.souls_used = bit32.bor(s.souls_used, used_bit(side, soul, s.offer_chaos))
end

-- The turn's start, before either side's change of form (0x08011DDC): the soul MegaMan is in reads the turn's
-- requests (ColonelSoul takes its arm chip, 0x080124AE).
function custom_souls.turn_opened(side: number)
    local part = united(side)
    if part ~= nil and part.turn_opened ~= nil then
        part.turn_opened(side, rules.state())
    end
end

-- A soul's button as the rules': on the screen while the side's navi is in the soul (`slots_of`), its functions
-- called with the rules' state.
local function button_of(part: SoulCustom, button: SoulButton): ButtonSpec
    local state, chip = button.state, button.chip
    return {
        slot = button.slot,
        cells = button.cells,
        uses = button.uses,
        right = button.right,
        left = button.left,
        shown = function(side: number): boolean
            return slots_of(side) == part
        end,
        state = if state ~= nil
            then function(side: number): "selectable" | "unavailable"
                return state(side, rules.state())
            end
            else nil,
        pressed = function(side: number)
            button.pressed(side, rules.state())
        end,
        chip = if chip ~= nil
            then function(side: number): Chip?
                return chip(side, rules.state())
            end
            else nil,
    }
end

-- A soul's window as the rules'.
local function window_of(window: SoulWindow): WindowSpec
    return {
        view = window.view,
        update = function(side: number): boolean
            return window.update(side, rules.state())
        end,
    }
end

-- Soul Unison's buttons and windows, by name (a button's is the key of its look in a pack), and the state fields
-- they keep beside the soul button's (rules/init.luau declares those): the soul button, and what each soul adds to the screen,
-- gathered from the souls of the navi that has them (MegaMan's `forms.souls`, each form's `custom`). A name is one of
-- Soul Unison's: two souls can't both add a button, a window or a screen field of the same name. (`screen`, above:
-- `forget_screen` reads it at each deal.)
local buttons: { [string]: ButtonSpec } = { soul = custom_souls.button }
local windows: { [string]: WindowSpec } = { soul_unison = custom_souls.window }
local forms = assert(megaman.forms)
for _, soul in assert(forms.souls) do
    local part: SoulCustom? = soul.custom
    if part ~= nil then
        local soul_buttons: { [string]: SoulButton } = part.buttons or {}
        for name, button in soul_buttons do
            if buttons[name] ~= nil then
                error(string.format("%s's button `%s` is another's too: a button's name is one of rules/souls's", tostring(soul.id), name))
            end
            buttons[name] = button_of(part, button)
        end
        local soul_windows: { [string]: SoulWindow } = part.windows or {}
        for name, window in soul_windows do
            if windows[name] ~= nil then
                error(string.format("%s's window `%s` is another's too: a window's name is one of rules/souls's", tostring(soul.id), name))
            end
            windows[name] = window_of(window)
        end
        local state: StateSchema = part.state or {}
        for name, kind in state do
            if screen[name] ~= nil then
                error(string.format("%s's screen field `%s` is another's too: a field's name is one of rules/souls's", tostring(soul.id), name))
            end
            screen[name] = kind
        end
    end
end
custom_souls.buttons = buttons
custom_souls.windows = windows
custom_souls.screen = screen

return custom_souls
