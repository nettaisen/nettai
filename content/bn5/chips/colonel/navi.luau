--!strict
-- Colonel, the navi of BN5's Colonel chips and CrossDiv (BN5's actor object #0x08, 0x080BA280; BN5's own, built on
-- BN6's navi template: BN6's Colonel, @bn6/chips/colonel, is another routine), brought by their navi chip controller.
-- He appears with his cape (his body overlay), picks his target as he comes (the first panel ahead holding an enemy
-- navi, else three columns ahead, else none), stands 20 ticks and (on a solid panel, with a target) draws his sword
-- and divides the screen there: one diagonal slash rising toward his front, or, if his user held B and Left while he
-- came, the one falling (with a sound); CrossDiv's both, an X. A flash and a camera shake. 20 ticks on he sheathes
-- it; he leaves, and the controller moves on.
--
-- Not reached: the slash of his Param1 0 (region 0x1F, effect 0x48): his init sets 2, and only 1 replaces it.

local navi_chips = require("@common/navi-chips/navi")
local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local regions = require("@common/regions")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- The chip's level (its Param1: 0, 4 for DS), which picks his palette and his sword's.
    level: number,
    -- CrossDiv's X (its Param3).
    cross: boolean,
    -- His slash (his Param2): 2 rising, 1 falling.
    slash: number,
}

local colonel = {}

local SPRITE = asset.sprite("bn5:colonel")
local SOUND_APPEAR = asset.sound("bn5:appear")
local SOUND_COMMAND = asset.sound("bn5:ok-8b")
local SOUND_SWING = asset.sound("bn5:spin")
-- What he wears (0x0800EE5E: BN5's body overlay row 3, its sprite 10-1E, in front at every animation), stepping
-- even while paused.
local CAPE = define.identity {
    id = "bn5:colonel-navi",
    class = "navi",
    parts = { body = { sprite = asset.sprite("bn5:colonel-cape"), anims = 41 } },
}
-- His sword (the attachment row 3), its animation 7, animating while dimmed; DS's palette 6 more.
local SWORD = attachment.look { sprite = asset.sprite("bn5:sword"), palette = 0 }
local SWORD_ANIM = 7
-- By the level (0x0800DA8A: navi 7's count, 2, by 0x0800DAE8's row): DS's 6 (SP's level is 0).
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 4, 6 }
-- His slashes by his Param2 (0x080BA49C: region, effect row 0x48 + it): rising, falling.
local SLASH_REGIONS: { [number]: Region } = { [1] = regions.falling, [2] = regions.rising }
local SLASHES: { [number]: Effect } = {
    [1] = define.effect { sprite = asset.sprite("bn5:screen-divide"), anim = 1, palette = 0 },
    [2] = define.effect { sprite = asset.sprite("bn5:screen-divide"), anim = 2, palette = 0 },
}
-- The panels his target holds (0x080BA5B0): an enemy navi's body, by side.
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local SOLID = 0x10
-- When he strikes, and when his swing's sound plays (ticks left).
local STRIKE, SWING = 0xC, 0x11

-- 0x080BA54C: his target: from the panel ahead along his row, the first holding an enemy navi; else three columns
-- ahead, if on the field; else none (0).
local function find_target(me: Object): (number, number)
    local y = me.panel_y
    local x = me.panel_x + me.facing
    while field.valid(x, y) do
        if bit32.btest(field.flags(x, y), ENEMY_NAVI[me.alliance]) then
            return x, y
        end
        x += me.facing
    end
    x = me.panel_x + me.facing * 3
    if not field.valid(x, y) then
        x = 0
    end
    return x, y
end

-- 0x080BA52E: his user holds B and Left (not for CrossDiv), once: the falling slash, with a sound.
local function command(me: Object)
    local s = me.state :: State
    if s.cross or s.slash == 1 then
        return
    end
    local user = me.related1 :: Object
    if user:held("left") and user:held("b") then
        battle.play_sound(SOUND_COMMAND)
        s.slash = 1
    end
end

-- Enter the phase with a timer (counting down from its first tick), or count down; true once it has run out.
local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- Action 0 (0x080BA320): he appears (the command read each tick); 4 ticks.
local function appear(me: Object)
    command(me)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
        me.timer2 = 0
    end
    if countdown(me, 4) then
        me:set_action(4)
    end
end

-- Action 4 (0x080BA368): 20 ticks standing (the command read each tick); then the strike, or he goes off a panel
-- that isn't solid or with no target.
local function stand(me: Object)
    command(me)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 0x14) then
        local strike = field.check(me.panel_x, me.panel_y, SOLID, 0) and me.future_panel_x ~= 0
        me:set_action(if strike then 8 else 0xC)
    end
end

-- 0x080BA414: a slash on his target (a hit region of the slash's diagonal: no spark, target 5, self 7, hit
-- modifier 3, running while dimmed) and its effect there.
local function slash(me: Object, n: number)
    local x, y = me.future_panel_x, me.future_panel_y
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 0,
        z = 0,
        region = SLASH_REGIONS[n],
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.slash,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0), SLASHES[n], bit32.bxor(me.alliance, me.flip))
end

-- Action 8, phase 0 (0x080BA3D0): his sword out (animation 5); its sound at 17 left; at 12 the slash (CrossDiv's
-- falling then rising), the camera shaking (3, 35 ticks) and the screen flashing (20 ticks; CrossDiv's 40); 21 ticks.
local function strike(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = 5, 0xFF
        attachment.attach(me, SWORD, "related", {
            anim = SWORD_ANIM,
            while_dimmed = true,
            palette_add = if s.level == 4 then 6 else 0,
        })
        me.timer = 0x15
        me.phase_init = 4
    end
    local t = me.timer
    if t == SWING then
        battle.play_sound(SOUND_SWING)
    end
    if t == STRIKE then
        if s.cross then
            s.slash = 1
        end
        slash(me, s.slash)
        if s.cross and s.slash ~= 2 then
            s.slash = 2
            slash(me, s.slash)
        end
        battle.shake_camera(3, 0x23)
        battle.palette_flash(0, if s.cross then 0x28 else 0x14, true, false)
    end
    local left = t - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- Action 8, phase 4 (0x080BA4AC): 20 ticks; his sword away (his first related slot cleared), standing.
local function sheathe(me: Object)
    if countdown(me, 0x14) then
        me.related1 = nil
        me.anim, me.anim_loaded = 0, 0xFF
        me:set_action(0xC)
    end
end

local ATTACK = { [0] = strike, [4] = sheathe }

local function attack(me: Object)
    local f = ATTACK[me.phase]
    if not f then
        error(string.format("Colonel's attack: phase %#x reads past its table (0x080BA3C8)", me.phase))
    end
    f(me)
end

-- Action 0xC (0x080BA4D8): 4 ticks; his cape off; the controller moves on, and he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 4) then
        me:remove_parts(CAPE)
        navi_chips.leave(me, (me.state :: State).controller)
    end
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = attack, [0xC] = leave }

-- 0x080BA2A4: his sprite (animation 3, his shadow), his palette by the level, facing his side's way, on his panel,
-- on the ground; his target; his slash rising; his cape.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 3, 3
    sprite:set_animation(3)
    sprite.palette = PALETTES[s.level] or 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.future_panel_x, me.future_panel_y = find_target(me)
    s.slash = 2
    me:add_parts(CAPE, 1)
    me:set_lifecycle("update")
end

colonel.kind = define.kind {
    id = "bn5:colonel/navi",
    pool = "actor",
    state = { controller = "object", level = "u8", cross = "bool", slash = "u8" },
    -- 0x080BA280.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("Colonel action %#x reads past his table (0x080BA310)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BA508: Colonel for `user`, reporting to `controller` (the summon table's entry 10), of a chip of `level`
-- (CrossDiv's: `cross`).
function colonel.summon_at(level: number, cross: boolean): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(colonel.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        s.cross = cross
        return o
    end
end

-- Colonel's and ColonelSP's (their Param1s 0), ColonelDS's (4), CrossDiv's (its Param3 1).
colonel.summon = colonel.summon_at(0, false)
colonel.summon_sp = colonel.summon_at(0, false)
colonel.summon_dark = colonel.summon_at(4, false)
colonel.summon_cross = colonel.summon_at(0, true)

return colonel
