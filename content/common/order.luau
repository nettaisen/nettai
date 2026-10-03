--!strict
-- A shuffle's order (`battle.shuffle_order`) put into a list.

local order = {}

-- `list`'s entries from `from` + 1 on in the shuffle's `new` order (each place's entry by its index from 1 before).
function order.apply<T>(list: { T }, from: number, new: { number })
    local before = table.clone(list)
    for i, j in new do
        list[from + i] = before[from + j]
    end
end

return order
