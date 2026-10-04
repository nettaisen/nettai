--!strict
-- Z Saver: three slashes with growing reach (the panel ahead and the next,
-- the column of three, three in a row); pressing B and Left together
-- (within three ticks) in the twelve ticks after the third slash's hit
-- gives a fourth, which throws a piercing sonic boom (BN6's action 0x5B,
-- `sub_80EFEE0`, docs/engine/shot-chips.md §13; BN5's 0x34, 0x080EE02A).
-- Left is the raw key, "back" for either side. Made for a game's look
-- (`zsaver.make`).
--
-- BN5's adds the Katanas' step (BN5's lib/stepsword): a soul's charged Z
-- Saver steps first (phase 0, marking its holder moving) and back after
-- (phase 0xC), and leaves two afterimages on each swing's sixth tick; and
-- its command's window shuts for good once A isn't held (the third slash's
-- hit opens it again).

local attachment = require("@exelib/attachment")
local regions = require("@exelib/regions")

local zsaver = {}

export type State = {
    -- The swing's countdown (AIAttackVars+0x10).
    timer: number,
    -- Slashes to make (AIAttackVars+0x40), and whether the command window
    -- is open (+0x44: 1), or (BN5's) shut for good (0xFF).
    slashes: number,
    window: boolean,
    shut: boolean,
    -- The command (`sub_8012956`'s buffer at AIAttackVars+0x30): done, the
    -- next slot, the ticks counted, and the three slots' B and Left bits.
    done: boolean,
    slot: number,
    ticks: number,
    keys: { number },
    -- BN5's step: it turned round to step behind the enemy (+0x28).
    turned: number,
}

local ANIM_SWING = 5
local SLASHES = 3
-- The swing's ticks (less one), the count it hits on, BN5's afterimages'
-- count, the recovery's.
local SWING_TICKS, HIT_TICK, AFTERIMAGE_TICK, RECOVER_TICKS = 0x15, 0xC, 0x10, 5
-- The command's ticks, and its keys (B and Left, as the game's bits).
local COMMAND_TICKS = 0xC
local KEY_B, KEY_LEFT = 0x02, 0x20
local HIT_MOD = 1
local EFFECT_Z = 0x10_0000
-- The slashes by index (BN6's `byte_80F004C`, BN5's 0x080EE2D8): the
-- region hit.
local REGIONS = { [0] = regions.long, regions.wide, regions.line }
-- The fourth slash's boom: it staggers and goes through.
local BOOM: SonicBoom = { hit_mod = 3, pierces = true }
local AFTERIMAGE_SHADER = 0xFFE0

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

-- Z Saver's action in `look`'s game.
function zsaver.make(look: ZSaverLook): Action
    local step_sword = look.step
    -- The phases, on the attack's step: BN6's start, slash and recovery;
    -- BN5's step first and the step back before the recovery.
    local START = if step_sword then 0x4 else 0x0
    local SLASH = if step_sword then 0x8 else 0x4
    local STEP_BACK = 0xC
    local RECOVER = if step_sword then 0x10 else 0x8

    -- `sub_80EFF00` (BN5's 0x080EE110): the slash index (a byte store over
    -- the shots word's low byte), three slashes, the command cleared.
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
        s.shut = false
        me.step, me.step_init = SLASH, 0
    end

    -- `sub_80EFF28` (BN5's 0x080EE138): the command while its window is
    -- open; a slash's swing; on the count's 0xC its hit and effect (index 2
    -- opens the window), or the fourth's boom; at the swing's end the next
    -- slash, or the recovery (BN5's step back first, charged).
    local function slash(me: Object, s: State)
        if look.a_held_window and not me:held("a") then
            s.shut = true
        end
        if s.window and not s.shut and command(me, s) then
            s.slashes += 1
            battle.play_sound(look.command_sound)
        end
        if me.step_init == 0 then
            local held = me.overlay
            if held then
                held:set_lifecycle("destroy")
            end
            me:set_animation(ANIM_SWING)
            me:refresh_form_overlay()
            me:open_counter_window()
            battle.play_sound(look.swing_sound)
            -- `sub_80EBB34`'s blade (row 3 for subtype 0), into the overlay
            -- slot.
            attachment.attach(look.attachment, me, look.blade, "overlay", { anim = look.blade_anim(me) })
            s.timer = SWING_TICKS
            me.step_init = 4
        end
        if step_sword and s.timer == AFTERIMAGE_TICK and me.charged ~= 0 then
            step_sword.afterimage(me, ANIM_SWING)
            battle.afterimage(me, me.pos, {
                sprite = look.blade.sprite,
                anim = look.blade_anim(me),
                flip = step_sword.flip(me),
                color_shader = AFTERIMAGE_SHADER,
                lifetime = 0x1E,
                shadow = "with_sprite",
            })
        end
        if s.timer == HIT_TICK then
            local index = me.attack_count
            local x, y = me.panel_x + me.facing, me.panel_y
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            if index < 3 then
                if index == 2 then
                    s.window = true
                    s.shut = false
                end
                local region = REGIONS[index]
                battle.hitbox(me, {
                    panel_x = x,
                    panel_y = y,
                    element = me.attack_element,
                    z = 0,
                    region = region,
                    hit_effect = nil,
                    target = look.hits,
                    self_type = look.is,
                    damage = bit32.band(damage, 0xFFFF),
                    stamina = bit32.rshift(damage, 16),
                    hit_mod = HIT_MOD,
                })
                local cx, cy = field.center(x, y)
                battle.effect(Vec3.new(cx, cy, EFFECT_Z), look.effects[index + 1], bit32.bxor(me.alliance, me.flip))
            else
                look.boom(me, x, y, me.attack_element, damage, BOOM)
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
        elseif step_sword and me.charged ~= 0 then
            me.step, me.step_init = STEP_BACK, 0
        else
            me.step, me.step_init = RECOVER, 0
        end
    end

    -- `sub_80F0068` (BN5's 0x080EE330): no longer moving; 6 ticks, then the
    -- sword goes and back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            me:set_status("moving", false)
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        end
    end

    local state: StateSchema = {
        timer = "u16",
        slashes = "u32",
        window = "bool",
        shut = "bool",
        done = "bool",
        slot = "u8",
        ticks = "u8",
        keys = "u8[3]",
        turned = "u32",
    }

    return define.action {
        id = look.id,
        state = state,
        -- `sub_80EFEE0` (BN5's 0x080EE02A).
        update = function(me: Object, s: State)
            local step = me.step
            if step_sword and step == 0 then
                -- 0x080EE050: straight to the start, or for a charged Z
                -- Saver, the step (its holder marked moving).
                step_sword.start(me, s, true)
            elseif step == START then
                start(me, s)
            elseif step == SLASH then
                slash(me, s)
            elseif step_sword and step == STEP_BACK then
                -- 0x080EE2F4.
                step_sword.step_back(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("Z Saver phase %#x reads past its phase table", step))
            end
        end,
    }
end

return zsaver
