--!strict
-- Z Saver: three slashes with growing reach (the panel ahead and the next,
-- the column of three, three in a row); pressing B and Left together
-- (within three ticks) in the twelve ticks after the third slash's hit
-- gives a fourth, which throws a piercing sonic boom (the original's
-- action 0x5B, `sub_80EFEE0`; docs/engine/shot-chips.md §13). Left is the
-- raw key, "back" for either side.
--
-- This is the action; the chip is ./chip. (The original's weapon routine
-- 0x6E starts the same action.)

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local sonic_boom = require("../../lib/swords/sonic_boom")

local zsaver = {}

export type State = {
    -- The swing's countdown (AIAttackVars+0x10).
    timer: number,
    -- Slashes to make (AIAttackVars+0x40), and whether the command window
    -- is open (+0x44).
    slashes: number,
    window: boolean,
    -- The command (`sub_8012956`'s buffer at AIAttackVars+0x30): done, the
    -- next slot, the ticks counted, and the three slots' B and Left bits.
    done: boolean,
    slot: number,
    ticks: number,
    keys: { number },
}

zsaver.STATE = {
    timer = "u16",
    slashes = "u32",
    window = "bool",
    done = "bool",
    slot = "u8",
    ticks = "u8",
    keys = "u8[3]",
}

-- The phases, on the attack's step.
local START, SLASH, RECOVER = 0x0, 0x4, 0x8

local ANIM_SWING = 5
local SLASHES = 3
-- The swing's ticks (less one), the count it hits on, the recovery's.
local SWING_TICKS, HIT_TICK, RECOVER_TICKS = 0x15, 0xC, 5
-- The command's ticks, and its keys (B and Left, as the game's bits).
local COMMAND_TICKS = 0xC
local KEY_B, KEY_LEFT = 0x02, 0x20
local SWING_SOUND = asset.sound("z-saver")
local COMMAND_SOUND = asset.sound("ok-8b")
local HIT_MOD = 1
local EFFECT_Z = 0x10_0000
-- The slashes by index (`byte_80F004C`, `dword_80F0064`): the region hit
-- and the effect drawn over the panel ahead (effects 0x65 to 0x67).
local SLASH_SPRITE = asset.sprite("sword-slash")
local HITS = {
    [0] = { region = regions.long, effect = define.effect { sprite = SLASH_SPRITE, anim = 1, palette = 4 } },
    [1] = { region = regions.wide, effect = define.effect { sprite = SLASH_SPRITE, anim = 0, palette = 4 } },
    [2] = { region = regions.line, effect = define.effect { sprite = SLASH_SPRITE, anim = 3, palette = 4 } },
}
-- The fourth slash's boom: it staggers and goes through.
local BOOM: SonicBoom = { hit_mod = 3, pierces = true }

-- `sub_8012956`: once done, false; past its twelve ticks, done; else the
-- pressed keys into the next of three slots, true (and done) when the
-- three hold both B and Left.
local function command(me: Object, s: State): boolean
    if s.done then
        return false
    end
    local ticks = s.ticks + 1
    s.ticks = ticks
    if ticks > COMMAND_TICKS then
        s.done = true
        return false
    end
    local slot = s.slot
    s.slot = if slot + 1 >= 3 then 0 else slot + 1
    local keys = 0
    if me:pressed("b") then
        keys = bit32.bor(keys, KEY_B)
    end
    if me:pressed("left") then
        keys = bit32.bor(keys, KEY_LEFT)
    end
    s.keys[slot + 1] = keys
    local all = bit32.bor(s.keys[1], s.keys[2], s.keys[3])
    if bit32.band(all, KEY_B + KEY_LEFT) == KEY_B + KEY_LEFT then
        s.done = true
        return true
    end
    return false
end

-- `sub_80EFF00`: the slash index (a byte store over the shots word's low
-- byte), three slashes, the command cleared.
local function start(me: Object, s: State)
    me:set_status("using_action", true)
    me.attack_count = bit32.band(me.attack_count, 0xFF00)
    s.slashes = SLASHES
    s.timer = 0
    s.done, s.slot, s.ticks = false, 0, 0
    for i = 1, 3 do
        s.keys[i] = 0
    end
    s.window = false
    me.step, me.step_init = SLASH, 0
end

-- `sub_80EFF28`: the command while its window is open; a slash's swing;
-- on the count's 0xC its hit and effect (index 2 opens the window), or the
-- fourth's boom; at the swing's end the next slash, or the recovery.
local function slash(me: Object, s: State)
    if s.window and command(me, s) then
        s.slashes += 1
        battle.play_sound(COMMAND_SOUND)
    end
    if me.step_init == 0 then
        local held = me.overlay
        if held then
            held:set_lifecycle("destroy")
        end
        me:set_animation(ANIM_SWING)
        me:refresh_form_overlay()
        me:open_counter_window()
        battle.play_sound(SWING_SOUND)
        -- `sub_80EBB34`'s blade (row 3 for subtype 0), into the overlay
        -- slot.
        attachment.attach(me, parts.blades.sword, "overlay", { anim = parts.blade_anim(me) })
        s.timer = SWING_TICKS
        me.step_init = 4
    end
    if s.timer == HIT_TICK then
        local index = me.attack_count
        local x, y = me.panel_x + me.facing, me.panel_y
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        if index < 3 then
            if index == 2 then
                s.window = true
            end
            local hit = HITS[index]
            battle.hitbox(me, {
                panel_x = x,
                panel_y = y,
                element = me.attack_element,
                z = 0,
                region = hit.region,
                hit_effect = nil,
                target = collision.hits_navis,
                self_type = collision.slash,
                damage = bit32.band(damage, 0xFFFF),
                stamina = bit32.rshift(damage, 16),
                hit_mod = HIT_MOD,
            })
            local cx, cy = field.center(x, y)
            battle.effect(Vec3.new(cx, cy, EFFECT_Z), hit.effect, bit32.bxor(me.alliance, me.flip))
        else
            sonic_boom.throw(me, x, y, me.attack_element, damage, BOOM)
        end
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    local index = bit32.band(me.attack_count + 1, 0xFFFF)
    me.attack_count = index
    if index < s.slashes then
        me.step_init = 0
    else
        me.step, me.step_init = RECOVER, 0
    end
end

-- `sub_80F0068`: no longer moving; 6 ticks, then the sword goes and back
-- to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("moving", false)
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

zsaver.action = define.action {
    id = "z-saver/action",
    state = zsaver.STATE,
    -- `sub_80EFEE0`.
    update = function(me: Object, s: State)
        local step = me.step
        if step == START then
            start(me, s)
        elseif step == SLASH then
            slash(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("Z Saver phase %#x reads past off_80EFEF4", step))
        end
    end,
}

return zsaver
