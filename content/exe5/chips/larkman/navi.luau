--!strict
-- LarkMan, the navi of the LarkMan chips (EXE5's actor object #0x1E, 0x080BDBFC; EXE5's own), brought by their navi
-- chip controller. He appears, stands, rises out of sight and sends his swoop (./swoop) round the field; when it
-- has had its time, he goes, and the controller moves on. While he is there, his user holding B and pressing up,
-- right and down in turn (each within a second of the first) powers the swoop (0x080BDDF4), if it is one that takes
-- the command. Each chip states his palette and his swoop's variant (`larkman.summon`): the LarkMan chips' variant 5,
-- WildBird's 6 (the summon table's entry 23, 0x080BDDCA: his Param4 1), which takes no command.

local navi_chips = require("@exelib/navi_chips/navi")
local swoop = require("./swoop")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- His chip's palette (the original's by the chip's level, its Param1: 0x080BDC7C, LarkMnDS's 4 the dark one).
    palette: number,
    -- The command: the steps entered (+0x60), the ticks left to finish it (+0x64), and done (+5).
    command_steps: number,
    command_ticks: number,
    commanded: boolean,
    -- His chip's swoop.
    swoop: LarkSwoopVariant,
}

local larkman = {}

local SPRITE = asset.sprite("larkman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_COMMAND = asset.sound("ok-8b")

-- The command's directions in turn, held with B (0x080BDE54).
local COMMAND: { Key } = { "up", "right", "down" }

-- 0x080BDC20: on his panel, on the ground, in his chip's palette.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
    local s = me.state :: State
    me.sprite.palette = s.palette
    s.commanded = false
    s.command_steps, s.command_ticks = 0, 0
    me:set_lifecycle("update")
end

-- 0x080BDDF4: the command, read from his user's held buttons (none for a swoop that takes none: WildBird's).
local function command(me: Object, s: State)
    if not s.swoop.commanded or s.commanded then
        return
    end
    if s.command_steps ~= 0 then
        s.command_ticks -= 1
        if s.command_ticks <= 0 then
            s.command_steps, s.command_ticks = 0, 0
        end
    end
    local user = me.related1 :: Object
    if not user:held("b") then
        return
    end
    local key = COMMAND[s.command_steps + 1]
    if not (key and user:held(key)) then
        return
    end
    s.command_steps += 1
    if s.command_steps >= 3 then
        s.commanded = true
        battle.play_sound(SOUND_COMMAND)
    elseif s.command_steps == 1 then
        s.command_ticks = 0x3C
    end
end

-- Enter the phase with a timer (a word store clears Timer2 too), or count down; true once it has run out.
local function countdown(me: Object, ticks: number, word: boolean): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
        if word then
            me.timer2 = 0
        end
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

-- Action 0 (0x080BDCA8): he appears; 11 ticks.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if countdown(me, 0xB, true) then
        me:set_action(4)
    end
end

-- Action 4 (0x080BDCDE): 40 ticks standing.
local function stand(me: Object)
    if countdown(me, 0x28, true) then
        me:set_action(8)
    end
end

-- Action 8 (0x080BDD02): he rises (animation 4) for 30 ticks, out of sight at 19 left.
local function rise(me: Object)
    local s = me.state :: State
    command(me, s)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 4
        me.timer = 0x1E
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x13 then
        me.visible = false
    elseif left <= 0 then
        me:set_action(0xC)
    end
end

-- Action 0xC (0x080BDD3E): 100 ticks; at 80 left, his swoop; at the end he goes, and the controller moves on.
local function attack(me: Object)
    local s = me.state :: State
    command(me, s)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = 0x64
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x50 then
        swoop.spawn(me, s.palette, me.element, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), s.swoop)
    elseif left <= 0 then
        local controller = s.controller
        if controller then
            navi_chip.navi_left(controller)
        end
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = rise, [0xC] = attack }

larkman.kind = new.kind {
    id = "larkman/navi",
    pool = "actor",
    state = {
        controller = "object",
        palette = "u8",
        command_steps = "u32",
        command_ticks = "i32",
        commanded = "bool",
        swoop = "record:larkman-swoop-variant",
    },
    -- 0x080BDBFC.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("LarkMan action %#x reads past his table (0x080BDC98)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- What a chip's LarkMan is: his palette and his swoop's variant.
export type Summon = { palette: number, swoop: LarkSwoopVariant }

-- 0x080BDDA0: LarkMan for `user`, reporting to `controller` (the summon table's entry 17; WildBird's 23, 0x080BDDCA),
-- as his chip's `summon` says.
function larkman.summon(summon: Summon): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(larkman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.palette = summon.palette
        s.swoop = summon.swoop
        return o
    end
end

return larkman
