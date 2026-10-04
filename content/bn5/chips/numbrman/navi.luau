--!strict
-- NumberMan, the navi of the NumberMan chips (BN5's actor object #0x0F, 0x080BB34C; BN5's own, built on BN6's navi
-- template), brought by their navi chip controller. He appears with his face (./overlay) (and goes at once off a
-- panel that isn't solid), stands 40 ticks, and throws a die (chips/numbrman/dice) three columns ahead: its face, drawn
-- (0x080BB4D8: 1 and 2 two times in 16, 3 to 5 three, 6 two... as the table goes), times his damage is what it
-- deals when it bursts there. Once it is gone he waits 30 ticks and leaves; the controller moves on.

local navi_chips = require("@common/navi-chips/navi")
local overlay = require("./overlay")
local dice = require("./dice")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- The chip's level (its Param1: 0, 3 for SP, 4 for DS), which picks his palette.
    level: number,
    -- His face (+0x60).
    face: Object?,
    -- His die is out (+0x0C: the flag he gave it).
    busy: boolean,
}

local numbrman = {}

local SPRITE = asset.sprite("bn5:numberman")
local SOUND_APPEAR = asset.sound("bn5:appear")
local SOUND_THROW = asset.sound("bn5:minibomb-throw")
-- By the level (0x0800DA8A: navi 9's count, 1, by 0x0800DAE8's row): SP's 2, DS's 3.
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 2, 3 }
-- The die's faces by a draw's low 4 bits (0x080BB4D8).
local FACES = { 1, 1, 2, 2, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 6, 6 }
-- The die (0x080BB4C2's word 0x30014): 20 HP, no status, hit modifier 3; 24 pixels above him.
local DIE_HP, DIE_HIT_MOD = 0x14, 3
local DIE_Z = 0x18_0000

-- Enter the phase with a timer (counting down from its first tick), or count down; true once it has run out (to
-- 0, or below 0 with `below`).
local function countdown(me: Object, ticks: number, below: boolean?): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if below then
        return left < 0
    end
    return left <= 0
end

-- Action 0 (0x080BB3E8): he appears; 4 ticks; then he stays, or goes off a panel that isn't solid.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.visible = true
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
    end
    if countdown(me, 4) then
        me:set_action(if field.solid(me.panel_x, me.panel_y) then 4 else 0xC)
    end
end

-- Action 4 (0x080BB426): 40 ticks standing.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 0x28) then
        me:set_action(8)
    end
end

-- Action 8, phase 0 (0x080BB468): his throw (animation 6, its sound); 11 ticks on, the die, three columns ahead,
-- 24 pixels above him (as he holds it).
local function throw(me: Object)
    if me.phase_init == 0 then
        me.anim = 6
        battle.play_sound(SOUND_THROW)
    end
    if not countdown(me, 0xA, true) then
        return
    end
    local face = FACES[bit32.band(battle.rng_positive(), 0xF) + 1]
    local p = me.pos
    dice.spawn(me, Vec3.new(p.x, p.y, p.z + DIE_Z), {
        face = face,
        target_x = me.panel_x + me.facing * 3,
        target_y = me.panel_y,
        dimmed = true,
        hp = DIE_HP,
        status = nil,
        hit_mod = DIE_HIT_MOD,
        damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)),
        holder = me,
    })
    -- (A halfword store: the phase's entry flag too.)
    me.phase, me.phase_init = 4, 0
end

-- Action 8, phase 4 (0x080BB4E8): once his die is gone, 31 ticks; then he leaves.
local function wait(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        if s.busy then
            return
        end
        me.timer = 0x1E
        me.phase_init = 4
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me:set_action(0xC)
    end
end

-- Action 0xC (0x080BB512): 4 ticks; then the controller moves on, his face goes, and he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 4) then
        local s = me.state :: State
        if s.controller then
            navi_chip.navi_left(s.controller :: Object)
        end
        local face = s.face
        if face then
            face:set_lifecycle("destroy")
        end
        -- (0x080BB53C: the state byte alone; his action and phase stay.)
        me.lifecycle = "destroy"
    end
end

local ATTACK = { [0] = throw, [4] = wait }

local function attack(me: Object)
    local f = ATTACK[me.phase]
    if not f then
        error(string.format("NumberMan's attack: phase %#x reads past its table (0x080BB460)", me.phase))
    end
    f(me)
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = attack, [0xC] = leave }

-- 0x080BB370: his sprite (animation 0, his shadow), his palette by the level, facing his side's way, his face on
-- him; on his panel, on the ground.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite.palette = PALETTES[s.level] or 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    s.face = overlay.spawn(me)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me:set_lifecycle("update")
end

numbrman.kind = define.kind {
    id = "bn5:numbrman/navi",
    pool = "actor",
    state = { controller = "object", level = "u8", face = "object", busy = "bool" },
    -- 0x080BB34C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("NumberMan action %#x reads past his table (0x080BB3D8)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BB540: NumberMan for `user`, reporting to `controller` (the summon table's entry 0), of a chip of `level`.
function numbrman.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(numbrman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

numbrman.summon = numbrman.summon_at(0)
numbrman.summon_sp = numbrman.summon_at(3)
numbrman.summon_dark = numbrman.summon_at(4)

return numbrman
