--!strict
-- EXE4's towers' controller (attack object 0x93, 0x080DF7F8: EXE4's own), which the spawners' variant 0x0C puts out
-- (0x080DF98C: its element and damage word the attack's, no position): AquaUp's, GreenWd's and LigtNin's, each its own
-- (`towers.controller`) with its strike, which the original picks by the chip's parameter (+0x10, 0x080DF898). From the
-- user's back column toward the other side it sweeps a column every 11 ticks (its row's 10, 0x080DF7F4, and one), each
-- of the column's panels by the strike; a column with nothing to rise from is passed over in the same tick; off the
-- field it goes.

local towers = {}

-- A panel's turn: whether something rose from it.
type Strike = (me: Object, x: number, y: number) -> boolean

-- The other side's navi on the panel, by the user's side (0x080DF8F8, 0x080DF954: its body bit).
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }

-- The controller's damage word, which what it raises takes.
function towers.damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- Whether (x, y) is a panel of type `kind` of the other side's, holding its navi (AquaUp's and GreenWd's test).
function towers.enemy_panel(me: Object, x: number, y: number, kind: PanelType): boolean
    local p = field.panel(x, y)
    return p ~= nil and p.kind == kind and p.alliance ~= me.alliance and field.check(x, y, ENEMY_BODY[me.alliance], 0)
end

-- Its row's ticks between columns (0x080DF7F4: 10 for each).
local EVERY = 10

-- A chip's controller `id` of its strike: its spawner (0x080DF98C, the spawners' variant 0x0C), the controller with
-- the attack's element and damage word, of the user's side. (Its position is the registers, which nothing reads.)
function towers.controller(spec: { id: string, strike: Strike }): (user: Object, element: number, damage: number) -> ()
    local strike = spec.strike

    -- 0x080DF83C: when its count runs out, each column on from where it is until one has something rise (or it walks
    -- off the field: gone).
    local function tick(me: Object)
        if me.phase_init == 0 then
            me.timer = EVERY
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t >= 0 then
            return
        end
        me.phase_init = 0
        while true do
            me.panel_y = 1
            if not field.valid(me.panel_x, 1) then
                me:free()
                return
            end
            local rose = false
            while me.panel_y <= 3 do
                if strike(me, me.panel_x, me.panel_y) then
                    rose = true
                end
                me.panel_y += 1
            end
            me.panel_x += me.vel.x
            if rose then
                return
            end
        end
    end

    local kind = new.kind {
        id = spec.id,
        pool = "attack",
        -- 0x080DF7F8.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                -- 0x080DF818: from the user's back column, a column a sweep toward its facing (+0x40).
                me.panel_x = me.alliance * 5 + 1
                me.vel = Vec3.new(me.facing, me.vel.y, me.vel.z)
                me:set_lifecycle("update")
            elseif l == "update" then
                tick(me)
            else
                me:free()
            end
        end,
    }

    return function(user: Object, element: number, damage: number)
        local o = battle.spawn(kind, Vec3.new(user.panel_y, element, user.pos.z))
        if o then
            o.element = element
            o.alliance = user.alliance
            o:set_damage_word(damage)
        end
    end
end

return towers
