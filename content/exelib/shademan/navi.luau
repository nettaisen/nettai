--!strict
-- ShadeMan, the navi of the ShadeMan chips, as EXE5 and EXE4 share him (actor object #0x19: EXE5's 0x080BD0E8, EXE4's
-- 0x080BAD70; their own, the same code but for what the game's spec states), brought by his chip's controller. He
-- appears, and for 31 ticks his side's navi may put in a command; then he calls out, and 21 ticks later sends his
-- crush (the game's, @exelib/shademan/crush) from the panel ahead; 61 ticks later he bows and goes (EXE5's) or goes
-- (EXE4's).
--
-- His commands, read from his side's navi's new presses while the battle is dimmed (a key of each step, its steps
-- within 10 ticks of each other, EXE5's 0x080BD2B6, EXE4's 0x080BAF46), each give the crush its hit (`ShadeManHit`);
-- without one it carries the game's own (EXE5's nothing; EXE4's a move bug in a direction he draws as he appears).
--
-- Each game makes his kind (`shademan.make`) and spawns him its own way; each chip states his palette.

local shademan = {}

type State = {
    -- The controller waiting for him (the flag it gave him).
    controller: Object?,
    -- His chip's palette.
    palette: number,
    -- The commands' progress (+0x60, +0x68: the step and the ticks left for the next).
    steps: { number },
    ticks: { number },
    -- Whether a command is in (+0x70's low byte), and what the crush carries (+0x70: the crush's parameters).
    commanded: boolean,
    hit: ShadeManHit,
}

local STEP_TICKS = 0xA

-- What the crush will carry, kept.
local function carry(s: State, hit: ShadeManHit)
    local h = s.hit
    h.palette, h.hit_mod, h.status = hit.palette, hit.hit_mod, hit.status
    h.bug, h.bug_arg = hit.bug or 0, hit.bug_arg or 0
end

-- Count the timer down; true once it went below 0.
local function past_zero(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left < 0
end

function shademan.make(spec: ShadeManSpec): ShadeMan
    -- One command's next step: true once it is in (with EXE5's sound).
    local function command_step(navi: Object, s: State, i: number): boolean
        local steps = spec.commands[i].steps
        local n = s.steps[i]
        local any = false
        for _, key in steps[n + 1] do
            if navi:dimmed_pressed(key) then
                any = true
            end
        end
        if any then
            s.ticks[i] = STEP_TICKS
            s.steps[i] = n + 1
            if steps[n + 2] == nil then
                if spec.command_sound then
                    battle.play_sound(spec.command_sound)
                end
                return true
            end
            return false
        end
        if n ~= 0 then
            local left = s.ticks[i] - 1
            if left ~= 0 then
                s.ticks[i] = int.u8(left)
            else
                s.steps[i], s.ticks[i] = 0, 0
            end
        end
        return false
    end

    -- Action 0 (EXE5's 0x080BD1D4, EXE4's 0x080BAE80): the commands, until one is in; 31 ticks, then he calls out.
    local function listen(me: Object)
        local s = me.state :: State
        if not s.commanded then
            local navi = battle.player(me.alliance)
            if navi then
                for i, c in spec.commands do
                    if command_step(navi, s, i) then
                        s.commanded = true
                        carry(s, c.hit)
                        break
                    end
                end
            end
        end
        if past_zero(me) then
            me.anim = spec.call_anim
            battle.play_sound(spec.call)
            me.timer = 0x14
            me:set_action(4)
        end
    end

    -- Action 4 (EXE5's 0x080BD22C, EXE4's 0x080BAEDA): 21 ticks; then his crush from the panel ahead; 61 ticks.
    local function call(me: Object)
        if past_zero(me) then
            spec.crush.spawn(me, me.panel_x + me.facing, me.panel_y, (me.state :: State).hit)
            me.timer = 0x3C
            me:set_action(8)
        end
    end

    -- Action 8 (EXE5's 0x080BD25C, EXE4's 0x080BAF0A): then EXE5's bow (animation 4, 6 ticks); EXE4's he goes.
    local function bow(me: Object)
        if past_zero(me) then
            if spec.bows then
                me.anim = 4
                me.timer = 5
                me:set_action(0xC)
            else
                me:set_lifecycle("destroy")
            end
        end
    end

    -- Action 0xC (EXE5's 0x080BD278): then he goes.
    local function leave(me: Object)
        if past_zero(me) then
            me:set_lifecycle("destroy")
        end
    end

    local ACTIONS = { [0] = listen, [4] = call, [8] = bow, [0xC] = if spec.bows then leave else nil }

    local function act(me: Object)
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("ShadeMan action %#x reads past his table", me.action))
        end
        action(me)
    end

    -- EXE5's 0x080BD11A, EXE4's 0x080BADB4: appearing on his panel with his shadow, his chip's palette; 30 ticks to
    -- put in a command; what the crush carries without one (EXE4's drawn now).
    local function init(me: Object)
        local s = me.state :: State
        local sprite = me.sprite
        sprite:load(spec.sprite)
        sprite.shadow = "ground"
        me.visible = true
        me.anim, me.anim_loaded = spec.appear_anim, spec.appear_anim
        sprite:set_animation(spec.appear_anim)
        sprite.palette = s.palette
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me:set_coordinates_from_panel()
        -- (EXE4's init keeps the spawner's height, its 0.)
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        battle.play_sound(spec.appear)
        me.timer = 0x1E
        for i = 1, #spec.commands do
            s.steps[i], s.ticks[i] = 0, 0
        end
        s.commanded = false
        carry(s, spec.uncommanded())
        me:set_lifecycle("update")
        act(me)
    end

    local kind = new.kind {
        id = spec.id,
        pool = "actor",
        state = {
            controller = "object",
            palette = "u8",
            steps = "u8[2]",
            ticks = "u8[2]",
            commanded = "bool",
            hit = { palette = "u8", hit_mod = "u8", status = "status", bug = "u8", bug_arg = "u8" } :: StateSchema,
        },
        -- EXE5's 0x080BD0E8, EXE4's 0x080BAD70.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                act(me)
            else
                -- EXE5's 0x080BD10C, EXE4's 0x080BAD94: EXE4's puff 16 pixels above him (effect row 0x13); the
                -- controller moves on, and he is freed.
                if spec.puff then
                    local p = me.pos
                    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), spec.puff)
                end
                local controller = (me.state :: State).controller
                if controller then
                    navi_chip.navi_left(controller)
                end
                me:free()
            end
            me:update_sprite_while_dimmed()
        end,
    }

    -- ShadeMan `o` (just spawned) reports to `controller`, in `palette`.
    local function summoned(o: Object, controller: Object, palette: number)
        local s = o.state :: State
        s.controller = controller
        s.palette = palette
    end

    return { kind = kind, summoned = summoned }
end

return shademan
