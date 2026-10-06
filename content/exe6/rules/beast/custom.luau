--!strict
-- Beast Out on EXE6's custom screen (docs/design/rules-in-luau.md §4.4, S6):
-- the button in the special slot (`sub_8029FB4`, `sub_802A57E`,
-- `sub_8028D6C`), its animation as a window (`sub_802770C`), the form it
-- puts the navi in at the turn's start (`sub_802937A`, `sub_802A040`), and
-- the round's record of it (`dword_20349A0`). Its state is the rules'.

local beast_out_chip = require("../../chips/beastout").beastout
local exe6 = require("../api")
local kind = require("../forms/kind")
local chip_gate_battle = require("../../lib/chip_gate_battle")

local custom_beast = {}

-- The rules' state fields Beast Out on the screen keeps (rules/init.luau
-- declares them).
type State = {
    -- `dword_20349A0`: Beast Out was picked this round (the round's first
    -- screen forgets it).
    beast_out_used: boolean,
}

-- The camera shake and the fades' speed of the animation (`sub_802774C`).
local SHAKE_MAGNITUDE, SHAKE_TICKS = 1, 0x28
local FADE_SPEED = 8
-- Its sounds with the pick's (`sub_802774C`, and the BeastOut chip's `sub_8027624`): the first by the Beast's game
-- (Gregar's on a Gregar console; the original's 0x191 and 0x193), then the flash (0xBC).
local CHOSEN_SOUNDS = { gregar = asset.sound("beast-out-chosen-gregar"), falzar = asset.sound("beast-out-chosen-falzar") }
local FLASH_SOUND = asset.sound("blinding-flash")

-- The side's navi changes form (MegaMan).
local function megaman(side: number): boolean
    return battle.navi(side).navi.forms ~= nil
end

-- The rules' setup (rules/init.luau declares it): the
-- player's game, and Beast Out unlocked (event flag 0xE0).
type Setup = {
    version: "falzar" | "gregar",
    beast_out: boolean,
}

-- Event flag 0x163, a navi code received (`sub_800B144` sends its level
-- only then): no Beast Out button.
local function sealed(side: number): boolean
    return exe6.navi_level(side) ~= nil
end

-- The game of the Beast the navi goes into, or is in: a form that isn't
-- the base form is its own version's (a Cross's, or a Beast form of one:
-- the other game's when the player's Crosses hold one of its, which a
-- save's never do; docs/engine/custom-screen.md §4.1), and the base form
-- the player's.
local function beast_game(side: number): "gregar" | "falzar"
    local form = battle.navi(side).form
    if form.version ~= nil and not form.base then
        return form.version :: "gregar" | "falzar"
    end
    return (rules.setup() :: Setup).version
end

-- The form Beast Out takes the navi to (`sub_802937A`, `sub_802A040`): when
-- `tired`, Beast Over (of `beast_game`'s game); from the base form the
-- version's Beast Out; from a Cross that Cross's form in Beast Out.
local function beast_form(side: number, tired: boolean): FormDef?
    local stats = battle.navi(side)
    local forms = stats.navi.forms
    if forms == nil then
        return nil
    end
    local function of(game: "gregar" | "falzar"): FormSet?
        return if game == "gregar" then forms.gregar else forms.falzar
    end
    if tired then
        local set = of(beast_game(side))
        return if set ~= nil then set.beast_over else nil
    elseif stats.form.base then
        local set = of((rules.setup() :: Setup).version)
        return if set ~= nil then set.beast_out else nil
    end
    return stats.form.beast
end

-- `sub_802A57E`: the navi can Beast Out now: not worn out, not in a Beast
-- form; tired, only once it has this round (and then goes Beast Over).
local function available(side: number): boolean
    local s = rules.state() :: State
    local emotion = custom.player(side).emotion
    return emotion ~= "worn_out" and (emotion ~= "tired" or s.beast_out_used) and not kind.beast(side)
end

-- The Beast Out button (`sub_8029FB4`, battle mode 0), in the special slot.
-- The BeastOut chip shows its picture in the chip window (`sub_802871C`).
custom_beast.button = {
    slot = 11,
    view = "chip_picture",
    shown = function(side: number): boolean
        return megaman(side)
            and not sealed(side)
            and not chip_gate_battle()
            and not custom.player(side).random_battle
            and (rules.setup() :: Setup).beast_out
    end,
    -- `sub_8028F48`: with room left, no Cross chosen, and the navi able to.
    state = function(side: number): "selectable" | "unavailable"
        if not custom.full(side) and available(side) and not custom.form_taken(side, "beast_out") then
            return "selectable"
        end
        return "unavailable"
    end,
    -- `sub_8028D6C`: picked, and the animation runs; the column shows the
    -- BeastOut chip (`sub_802A034`).
    pressed = function(side: number)
        if custom.cursor_state(side) ~= "selectable" or custom.full(side) then
            custom.refuse(side)
            return
        end
        custom.pick(side)
        custom.play(side, "pick")
        custom.open_window(side, "beast_out")
        custom.set_column_icon(side, beast_out_chip)
    end,
    -- `sub_802A0EC`: taking Beast Out back takes its face back.
    taken_back = function(side: number)
        custom.set_form(side, "beast_out", nil)
        custom.set_face(side, nil)
        custom.play(side, "back")
        custom.play(side, "cancel")
    end,
} :: ButtonSpec

-- The BeastOut chip picked from a chip slot (`sub_802A00C`): its
-- animation (`sub_80275EC`).
function custom_beast.chip_picked(side: number, chip: Chip)
    if chip == beast_out_chip then
        custom.open_window(side, "beast_out_chip")
    end
end

-- `sub_802A0EC`: taking the BeastOut chip back takes its face back.
function custom_beast.chip_taken_back(side: number, chip: Chip)
    if chip == beast_out_chip then
        custom.set_face(side, nil)
        custom.play(side, "back")
        custom.play(side, "cancel")
    end
end

-- `sub_80275EC`: the BeastOut chip picked, 85 ticks: like Beast Out's, 16
-- ticks later: its fade out (0x64) starts at tick 17, with sounds 0x193 and
-- 0xBC; 50 ticks on (tick 68) the chip moves to the front of the selection
-- (the Beast Out button's state and the form stay as they are: the hand's
-- chip is the Beast Out), and the fade back in (0x60) ends it.
custom_beast.chip_window = {
    update = function(side: number): boolean
        local tick = custom.window_tick(side)
        if tick == 1 then
            -- sub_8027618
            custom.set_frame(side, 0)
        elseif (tick >= 2 and tick <= 16) or (tick >= 18 and tick <= 67) then
            -- sub_8027624
            custom.set_frame(side, custom.frame(side) + 1)
        elseif tick == 17 then
            -- sub_8027624's last tick: this console's camera shakes as for
            -- Beast Out, and the screen fades.
            custom.shake(side, SHAKE_MAGNITUDE, SHAKE_TICKS)
            custom.set_frame(side, 0)
            custom.fade(side, "half_out", FADE_SPEED)
            custom.play_sound(side, CHOSEN_SOUNDS[beast_game(side)])
            custom.play_sound(side, FLASH_SOUND)
        elseif tick == 68 then
            -- sub_8027672
            custom.pick_first(side, nil)
            custom.set_face(side, beast_form(side, false))
            custom.update_availability(side)
            custom.fade(side, "half_out_back", FADE_SPEED)
        end
        custom.draw_emblem(side, 0)
        return tick < 85
    end,
}

-- `sub_802770C`: Beast Out picked, 70 ticks.
custom_beast.window = {
    update = function(side: number): boolean
        local tick = custom.window_tick(side)
        if tick == 1 then
            -- sub_8027738: the form's taken, and the emblem spins.
            custom.set_form(side, "beast_out", beast_form(side, custom.player(side).emotion == "tired"))
            custom.set_frame(side, 0)
            custom.spin(side)
        elseif tick == 2 then
            -- sub_802774C: the screen fades (0x64) and this console's
            -- camera shakes, 40 ticks at magnitude 1.
            custom.shake(side, SHAKE_MAGNITUDE, SHAKE_TICKS)
            custom.set_frame(side, 0)
            custom.fade(side, "half_out", FADE_SPEED)
            custom.play_sound(side, CHOSEN_SOUNDS[beast_game(side)])
            custom.play(side, "pick")
            custom.play_sound(side, FLASH_SOUND)
        elseif tick >= 3 and tick <= 52 then
            -- sub_802777C
            custom.set_frame(side, custom.frame(side) + 1)
        elseif tick == 53 then
            -- sub_8027796: the screen fades back in, and the emotion window
            -- shows the Beast form (sub_802A040). Beast Out goes first in
            -- the selection, so B takes it back last.
            custom.fade(side, "half_out_back", FADE_SPEED)
            custom.set_face(side, beast_form(side, custom.player(side).emotion == "tired"))
            custom.pick_first(side, beast_out_chip)
            custom.set_button_state(side, "beast_out", "selected")
            custom.update_availability(side)
        end
        custom.draw_emblem(side, 0)
        return tick < 70
    end,
}

-- The round's first screen forgets the round's Beast Out.
function custom_beast.open(side: number)
    if battle.turn() == 1 then
        (rules.state() :: State).beast_out_used = false
    end
end

-- OK with Beast Out picked: the round has had it.
function custom_beast.confirmed(side: number)
    if custom.button_picked(side, "beast_out") then
        (rules.state() :: State).beast_out_used = true
    end
end

return custom_beast
