--!strict
-- NapalmMan, the navi of BN5's NapalmMan chips (BN5's actor object #0x0E, 0x080BB10C; BN5's own, built on BN6's
-- navi template), brought by their navi chip controller, wearing his cannon (BN5's body overlay row 4). He appears
-- (a sound); off a solid panel he sinks and goes 11 ticks on. Else, 31 ticks on, he raises his cannon (animation
-- 18) for 31 ticks and fires (animation 19, a sound) a napalm bomb (chips/napalmmn/bomb) from 8 pixels ahead and 56
-- up at the panel four columns ahead in his row (the field's edge column at most), running while dimmed; 88 ticks
-- on he stands 30, sinks (animation 4) 5, and goes; the controller moves on.

local napalm_bomb = require("./bomb")
local navi_chips = require("@exelib/navi-chips/navi")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- The chip's level (its Param1: 0, 3 for SP, 4 for DS), which picks his palette.
    level: number,
}

local napalmman = {}

local SPRITE = asset.sprite("bn5:napalmman")
local SOUND_APPEAR = asset.sound("bn5:appear")
local SOUND_FIRE = asset.sound("bn5:cannon")
-- What he wears (0x0800EE8A: BN5's body overlay row 4, its sprite 10-18, behind him in his animations 20 to 22),
-- stepping even while paused.
local CANNON = define.identity {
    id = "bn5:napalmman-navi",
    class = "navi",
    parts = {
        body = { sprite = asset.sprite("bn5:napalm-cannon"), anims = 23, behind = { { 20, 22 } } },
    },
}
-- By the level (0x0800DA8A: navi 4's count, 3, by 0x0800DAE8's row): SP's 6, DS's 9.
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 6, 9 }
-- His bomb: how far ahead it lands (columns), and where it starts (16.16, ahead and up).
local REACH = 4
local MUZZLE_X, MUZZLE_Z = 0x8_0000, 0x38_0000

-- He goes: the controller moves on, his cannon comes off.
local function leave(me: Object)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:remove_parts(CANNON)
    me:set_lifecycle("destroy")
end

-- Action 0 (0x080BB1A8): shown; on a solid panel 31 ticks, then his attack; else he sinks (animation 4, 3 ticks
-- before the end) and goes 11 ticks on.
local function appear(me: Object)
    local solid = field.solid(me.panel_x, me.panel_y)
    if me.phase_init == 0 then
        me.timer, me.timer2 = if solid then 0x1E else 0xA, 0
        me.visible = true
        me.phase_init = 4
        return
    end
    if not solid and me.timer == 3 then
        me.anim = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    if field.solid(me.panel_x, me.panel_y) then
        me.timer, me.timer2 = 0x1E, 0
        me:set_action(4)
    else
        leave(me)
    end
end

-- Attack phase 0 (0x080BB234): his cannon up (animation 18), 31 ticks.
local function aim(me: Object)
    if me.phase_init == 0 then
        me:set_animation(0x12)
        me.timer = 0x1E
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- Attack phase 4 (0x080BB25A): he fires (animation 19, a sound): the bomb at the panel four columns ahead; 88
-- ticks.
local function fire(me: Object)
    if me.phase_init == 0 then
        me:set_animation(0x13)
        local tx = math.clamp(me.panel_x + me.facing * REACH, 0, 7)
        local p = me.pos
        napalm_bomb.lob(me, Vec3.new(int.i32(p.x + me.facing * MUZZLE_X), p.y, int.i32(p.z + MUZZLE_Z)), {
            variant = 1,
            target_x = tx,
            target_y = me.panel_y,
            damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)),
            while_dimmed = true,
        })
        battle.play_sound(SOUND_FIRE)
        me.timer = 0x58
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.phase, me.phase_init = 8, 0
    end
end

-- Attack phase 8 (0x080BB2C0): standing (animation 0), 30 ticks; then he sinks.
local function rest(me: Object)
    if me.phase_init == 0 then
        me:set_animation(0)
        me.timer = 0x1E
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_action(8)
    end
end

local ATTACK = { [0] = aim, [4] = fire, [8] = rest }

-- Action 4 (0x080BB214): his attack, by its phase.
local function attack(me: Object)
    local phase = ATTACK[me.phase]
    if not phase then
        error(string.format("NapalmMan's attack: phase %#x reads past its table (0x080BB228)", me.phase))
    end
    phase(me)
end

-- Action 8 (0x080BB2E8; its phase byte the entry flag): he sinks (animation 4), 5 ticks; hidden, he goes.
local function sink(me: Object)
    if me.phase == 0 then
        me.anim, me.anim_loaded = 4, 0
        me.timer = 5
        me.phase = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    me.visible = false
    leave(me)
end

local ACTIONS = { [0] = appear, [4] = attack, [8] = sink }

-- 0x080BB130: his sprite (animation 3, his shadow), shown, his palette by the level, facing his side's way; his
-- cannon on; on his panel, on the ground; a sound.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 3, 3
    sprite:set_animation(3)
    sprite.palette = PALETTES[s.level] or 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:add_parts(CANNON, 1)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    battle.play_sound(SOUND_APPEAR)
    me:set_lifecycle("update")
end

napalmman.kind = define.kind {
    id = "bn5:napalmmn/navi",
    pool = "actor",
    state = { controller = "object", level = "u8" },
    -- 0x080BB10C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("NapalmMan action %#x reads past his table (0x080BB19C)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BB324: NapalmMan for `user`, reporting to `controller` (the summon table's entry 7), of a chip of `level`.
function napalmman.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(napalmman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

-- NapalmMan's (his Param1 0), NapalmManSP's (3), NapalmManDS's (4).
napalmman.summon = napalmman.summon_at(0)
napalmman.summon_sp = napalmman.summon_at(3)
napalmman.summon_dark = napalmman.summon_at(4)

return napalmman
