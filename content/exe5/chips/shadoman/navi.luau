--!strict
-- ShadowMan, the navi of the ShadowMan chips (EXE5's actor object #0x07, 0x080B9E98; EXE5's own, built on EXE6's navi
-- template), brought by their navi chip controller. The controller's ShadowMan (his Param1 0) is unseen: he sends
-- three of himself (Param1 1), one to each row, at the first panel of his user's side from its back corner, 15 ticks
-- apart, each with a target (an enemy combatant ahead of it: the first not yet targeted, else the one with the most
-- HP), and waits for them; then the controller moves on. Each appears, stands 20 ticks (and its delay), vanishes in
-- a puff, rises 32 pixels, throws a shuriken (chips/shadoman/shuriken) at its target, waits 20 ticks and blinks out.

local navi_chips = require("@exelib/navi_chips/navi")
local shuriken = require("./shuriken")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54), or, for one he sent, him.
    controller: Object?,
    -- Which of his he is (the flag he gave it, 0 to 2).
    index: number,
    -- The chip's level (Param0: 0, 3 for SP, 4 for DS), which picks the palette.
    level: number,
    -- Sent by the unseen one (Param1), its delay (Param2) and its target, x | y << 4 (Param3).
    sent: boolean,
    delay: number,
    target: number,
    -- The unseen one's: the three he sent are out (+0x60).
    busy: { number },
}

local shadoman = {}

local SPRITE = asset.sprite("shadowman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_THROW = asset.sound("boomerang")
-- The effect table's row 2: the puff he vanishes in, 32 pixels up.
local PUFF = define.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 }
-- By the level (0x0800DA8A: navi 8's count, 1, by 0x0800DAE8's row): SP's 2, DS's 3.
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 2, 3 }
-- Where the three appear (0x080BA158): the first panel of his user's side, by side, as { require, forbid }.
local OWN_AREA = { [0] = { 0, 0x20 }, { 0x20, 0 } }
local RISE = 0x20_0000
local HIT_MOD = 3

-- `object_getFirstPanelInDirectionFiltered`: from (x, y) along `dir`, the first panel whose flags have every
-- `require` bit and no `forbid` bit (the first checked before the field's edge is); x 0 off the field.
local function first_panel(x: number, y: number, require: number, forbid: number, dir: number): (number, number)
    while true do
        if field.check(x, y, require, forbid) then
            return x, y
        end
        x += dir
        if not field.valid(x, y) then
            return 0, y
        end
    end
end

-- 0x080BA216: `x2` lies ahead of `x` for his facing.
local function ahead(me: Object, x: number, x2: number): boolean
    local d = x2 - x
    if bit32.bxor(me.alliance, me.flip) ~= 0 then
        d = -d
    end
    return d > 0
end

-- 0x080BA168: the target of the one sent to (x, y): of the enemy combatants ahead of it, the first not yet targeted
-- (now targeted), else the one with the most HP (the last of the most); 0 if none.
local function pick_target(me: Object, x: number, targeted: { [number]: boolean }): number
    local slots: { number } = {}
    local actors: { [number]: Object } = {}
    local list = battle.alive_actors(bit32.bxor(me.alliance, 1))
    for i, o in list do
        if ahead(me, x, o.panel_x) then
            table.insert(slots, i)
            actors[i] = o
        end
    end
    if #slots == 0 then
        return 0
    end
    for _, i in slots do
        if not targeted[i] then
            targeted[i] = true
            local o = actors[i]
            return bit32.bor(o.panel_x, bit32.lshift(o.panel_y, 4))
        end
    end
    local best = slots[1]
    local hp = actors[best].hp
    for k = 2, #slots do
        local i = slots[k]
        if actors[i].hp >= hp then
            best, hp = i, actors[i].hp
        end
    end
    local o = actors[best]
    return bit32.bor(o.panel_x, bit32.lshift(o.panel_y, 4))
end

-- 0x080BA0C6: the three, rows 3 to 1, at the first panel of his user's side from its back corner (none if the row
-- has none), 15 ticks apart.
local function send(me: Object)
    local s = me.state :: State
    local filter = OWN_AREA[me.alliance]
    local start = bit32.bxor(me.alliance, me.flip) * 5 + 1
    local delay = 0
    -- (+0x64: which of the enemies are targeted, by slot; the list holds the slots in use, in order.)
    local targeted: { [number]: boolean } = {}
    for i = 1, 3 do
        local row = 4 - i
        local x, y = first_panel(start, row, filter[1], filter[2], me.facing)
        if x ~= 0 then
            local target = pick_target(me, x, targeted)
            local user = me.related1 :: Object
            local o = battle.spawn(shadoman.kind, Vec3.new(y, me.element, s.level))
            if o then
                o.panel_x, o.panel_y = x, y
                o.element = me.element
                o.related1 = user
                o.alliance, o.flip = user.alliance, user.flip
                o:set_damage_word(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
                local c = o.state :: State
                c.controller = me
                c.index = i - 1
                c.level = s.level
                c.sent = true
                c.delay = delay
                c.target = target
                s.busy[i] = 1
            end
        end
        delay += 0xF
    end
end

-- 0x080BA0A0: the unseen one: he sends the three; once all are done, the controller moves on and he is freed.
local function unseen(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        for i = 1, 3 do
            s.busy[i] = 0
        end
        send(me)
        me.phase_init = 4
    end
    if s.busy[1] == 0 and s.busy[2] == 0 and s.busy[3] == 0 then
        local c = s.controller
        if c then
            navi_chip.navi_left(c)
        end
        me:free()
    end
end

-- One sent is done: its flag cleared.
local function sent_done(me: Object)
    local s = me.state :: State
    local master = s.controller
    if master then
        ((master.state :: any) :: State).busy[s.index + 1] = 0
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

-- Action 0 (0x080B9F2C): it appears; 4 ticks.
local function appear(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
        me.timer2 = 0
    end
    if countdown(me, 4) then
        me:set_action(4)
    end
end

-- Action 4 (0x080B9F5C): 20 ticks standing, and its delay.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, (me.state :: State).delay + 0x14) then
        me:set_action(8)
    end
end

-- Action 8, phase 0 (0x080B9FA8): the puff 32 pixels up; on its next tick, phase 4 (its 15-tick timer never read:
-- the branch past the entry skips it).
local function vanish(me: Object)
    if me.phase_init ~= 0 then
        me.phase, me.phase_init = 4, 0
        return
    end
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + RISE), PUFF)
    me.timer = 0xF
    me.phase_init = 4
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.phase, me.phase_init = 4, 0
    end
end

-- Action 8, phase 4 (0x080B9FD8): risen 32 pixels (animation 3); 3 ticks.
local function rise(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
        me.timer = 3
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, RISE)
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.phase, me.phase_init = 8, 0
    end
end

-- Action 8, phase 8 (0x080BA002): the throw (animation 12, its sound): a shuriken at its target (none for 0, hit
-- modifier 3, running while dimmed); 15 ticks.
local function throw(me: Object)
    if me.phase_init == 0 then
        me.anim = 0xC
        me.timer = 0xF
        battle.play_sound(SOUND_THROW)
        local t = (me.state :: State).target
        local x, y = bit32.band(t, 7), bit32.rshift(t, 4)
        if x ~= 0 then
            local o = shuriken.spawn(me, x, y, 0, HIT_MOD, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
            if o then
                o.run_while_dimmed = true
            end
        end
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.phase, me.phase_init = 0xC, 0
    end
end

-- Action 8, phase 0xC (0x080BA042): 20 ticks.
local function wait(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x14
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_action(0xC)
    end
end

local ATTACK = { [0] = vanish, [4] = rise, [8] = throw, [0xC] = wait }

local function attack(me: Object)
    local f = ATTACK[me.phase]
    if not f then
        error(string.format("ShadowMan's attack: phase %#x reads past its table (0x080B9F98)", me.phase))
    end
    f(me)
end

-- Action 0xC (0x080BA066): 30 ticks blinking (hidden while its timer's bit 1 is set); then it is done, and goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x1E
        me.phase_init = 4
    end
    me.visible = bit32.band(me.timer, 2) == 0
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        sent_done(me)
        -- (0x080BA09C: the state byte alone; its action and phase stay.)
        me.lifecycle = "destroy"
    end
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = attack, [0xC] = leave }

-- 0x080B9EC8: its sprite (animation 3, its shadow), its palette by the level, facing its side's way, on its panel,
-- on the ground.
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
    me:set_lifecycle("update")
end

shadoman.kind = define.kind {
    id = "shadoman/navi",
    pool = "actor",
    state = {
        controller = "object",
        index = "u8",
        level = "u8",
        sent = "bool",
        delay = "u8",
        target = "u8",
        busy = "u8[3]",
    },
    -- 0x080B9E98: the unseen one, whatever his lifecycle (no sprite); one sent, by its lifecycle.
    update = function(me: Object)
        if not (me.state :: State).sent then
            unseen(me)
            return
        end
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("ShadowMan action %#x reads past his table (0x080B9F1C)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BA25A: ShadowMan (the unseen one) for `user`, reporting to `controller` (the summon table's entry 11), of a
-- chip of `level`. (His position is the spawner's registers, which nothing moves: the row, the element and the
-- address the controller called him through.)
-- (The address is Team ProtoMan US's ROM's; compat/games.toml has the other ROMs'.)
function shadoman.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(shadoman.kind, user, spec, 0x080BA25B)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

shadoman.summon = shadoman.summon_at(0)
shadoman.summon_sp = shadoman.summon_at(3)
shadoman.summon_dark = shadoman.summon_at(4)

return shadoman
