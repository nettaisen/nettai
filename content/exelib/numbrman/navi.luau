--!strict
-- NumberMan, the navi of the NumberMan chips, as EXE5 and EXE4 share him (actor object #0x0F: EXE5's 0x080BB34C,
-- built on EXE6's navi template; EXE4's 0x080B87AC; the same code but for the game's spec), brought by his chip's
-- controller. He appears with his face (the game's, @exelib/numbrman/face) (and goes at once off a panel that isn't
-- solid), stands 40 ticks, and throws a die (the game's, @exelib/numbrman/dice) three columns ahead: its face, drawn
-- (EXE5's 0x080BB4D8, EXE4's 0x080B8930: 1 and 2 two times in 16, 3 to 5 three, 6 two... as the table goes), times
-- his damage is what it deals when it bursts there. Once it is gone he waits 30 ticks and leaves; the controller
-- moves on.
--
-- Each game makes its kind (`numbrman.make`) and spawns him its own way; each chip states his palette.

local numbrman = {}

type State = {
    -- The controller waiting for him (the flag it gave him: EXE5's +0x54, EXE4's +0x50).
    controller: Object?,
    -- His chip's palette.
    palette: number,
    -- His face (+0x60).
    face: Object?,
    -- His die is out (+0x0C: the flag he gave it).
    busy: boolean,
}

-- The die's faces by a draw's low 4 bits (EXE5's 0x080BB4D8, EXE4's 0x080B8930).
local FACES = { 1, 1, 2, 2, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 6, 6 }
-- His die rises from 24 pixels above him.
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

function numbrman.make(spec: NumberManSpec): NumberMan
    -- Action 0 (EXE5's 0x080BB3E8, EXE4's 0x080B8848): he appears; 4 ticks; then he stays, or goes off a panel that
    -- isn't solid.
    local function appear(me: Object)
        if me.phase_init == 0 then
            me.visible = true
            me.anim = 3
            battle.play_sound(spec.appear)
        end
        if countdown(me, 4) then
            me:set_action(if field.solid(me.panel_x, me.panel_y) then 4 else 0xC)
        end
    end

    -- Action 4 (EXE5's 0x080BB426, EXE4's 0x080B8886): 40 ticks standing.
    local function stand(me: Object)
        if me.phase_init == 0 then
            me.anim = 0
        end
        if countdown(me, 0x28) then
            me:set_action(8)
        end
    end

    -- Action 8, phase 0 (EXE5's 0x080BB468, EXE4's 0x080B88C8): his throw (animation 6, its sound); 11 ticks on, the
    -- die, three columns ahead, 24 pixels above him (as he holds it).
    local function throw(me: Object)
        if me.phase_init == 0 then
            me.anim = 6
            battle.play_sound(spec.throw)
        end
        if not countdown(me, 0xA, true) then
            return
        end
        local face = FACES[bit32.band(battle.rng_positive(), 0xF) + 1]
        local p = me.pos
        local die = spec.die
        spec.dice.spawn(me, Vec3.new(p.x, p.y, p.z + DIE_Z), {
            face = face,
            target_x = me.panel_x + me.facing * 3,
            target_y = me.panel_y,
            dimmed = true,
            hp = die.hp,
            status = die.status,
            hit_mod = die.hit_mod,
            damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)),
            holder = me,
        })
        -- (A halfword store: the phase's entry flag too.)
        me.phase, me.phase_init = 4, 0
    end

    -- Action 8, phase 4 (EXE5's 0x080BB4E8, EXE4's 0x080B8940): once his die is gone, 31 ticks; then he leaves.
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

    -- Action 0xC (EXE5's 0x080BB512, EXE4's 0x080B896A): 4 ticks; then the controller moves on, his face goes, and he
    -- goes.
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
            -- (The state byte alone: his action and phase stay.)
            me.lifecycle = "destroy"
        end
    end

    local ATTACK = { [0] = throw, [4] = wait }

    local function attack(me: Object)
        local f = ATTACK[me.phase]
        if not f then
            error(string.format("NumberMan's attack: phase %#x reads past its table", me.phase))
        end
        f(me)
    end

    local ACTIONS = { [0] = appear, [4] = stand, [8] = attack, [0xC] = leave }

    -- EXE5's 0x080BB370, EXE4's 0x080B87D0: his sprite (animation 0, his shadow), his chip's palette, facing his
    -- side's way, his face on him; on his panel, on the ground.
    local function init(me: Object)
        local s = me.state :: State
        local sprite = me.sprite
        sprite:load(spec.sprite)
        sprite.shadow = "ground"
        me.anim, me.anim_loaded = 0, 0xFF
        sprite.palette = s.palette
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        s.face = spec.face.spawn(me)
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        me:set_lifecycle("update")
    end

    local kind = new.kind {
        id = spec.id,
        pool = "actor",
        state = { controller = "object", palette = "u8", face = "object", busy = "bool" },
        -- EXE5's 0x080BB34C, EXE4's 0x080B87AC.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                local action = ACTIONS[me.action]
                if not action then
                    error(string.format("NumberMan action %#x reads past his table", me.action))
                end
                action(me)
            else
                me:free()
            end
            me:update_sprite_while_dimmed()
        end,
    }

    -- NumberMan `o` (just spawned) reports to `controller`, in `palette`.
    local function summoned(o: Object, controller: Object, palette: number)
        local s = o.state :: State
        s.controller = controller
        s.palette = palette
    end

    return { kind = kind, summoned = summoned }
end

return numbrman
