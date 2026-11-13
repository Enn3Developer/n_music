import SwiftUI

/// A rule being written (`qml/RuleRow.qml`): [Field 150] [Condition 110] [value editors] [×];
/// below 440 points of width the editors go on a second line.
struct RuleRowView: View {
    @Binding var rule: FilterRule
    let remove: () -> Void

    @State private var width: CGFloat = 480

    private var field: Filters.Field? { Filters.fields[rule.field] }

    var body: some View {
        let wrapped = width < 440
        Group {
            if wrapped {
                VStack(alignment: .leading, spacing: 8) {
                    HStack(spacing: 8) {
                        fieldBox
                        condition
                        Spacer(minLength: 0)
                        RuleRemoveButton(label: tr("remove_rule"), action: remove)
                    }
                    // Up to the "×" above, which stays on the first line (`qml/RuleRow.qml:70-74`).
                    editors
                        .frame(height: 36)
                        .padding(.trailing, 32 + 8)
                }
            } else {
                HStack(spacing: 8) {
                    fieldBox
                    condition
                    editors
                    RuleRemoveButton(label: tr("remove_rule"), action: remove)
                }
                .frame(height: 36)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            GeometryReader { geometry in
                Color.clear
                    .onAppear { width = geometry.size.width }
                    .onChange(of: geometry.size.width) { _, new in width = new }
            }
        )
    }

    private var fieldOptions: [RuleSelectOption] {
        Filters.fieldGroups.flatMap { group in
            [RuleSelectOption.heading(tr(group.headingKey))]
                + group.fields.map { RuleSelectOption(value: $0, label: Filters.label($0)) }
        }
    }

    private var fieldBox: some View {
        RuleSelectBox(
            options: fieldOptions,
            value: Binding(get: { rule.field }, set: { rule = Filters.retarget(rule, to: $0) }),
            display: Filters.shortName(rule.field),
            label: tr("filter_field")
        )
        .frame(width: 150)
    }

    @ViewBuilder
    private var condition: some View {
        if field?.kind == .period {
            Text(tr("op_the_last"))
                .font(.system(size: 13))
                .foregroundStyle(Theme.text3)
                .padding(.leading, 4)
                .frame(width: 110, alignment: .leading)
        } else {
            RuleSelectBox(
                options: (field?.ops ?? []).map { RuleSelectOption(value: $0, label: Filters.opLabel($0)) },
                value: $rule.op,
                muted: true,
                label: tr("filter_condition")
            )
            .frame(width: 110)
        }
    }

    @ViewBuilder
    private var editors: some View {
        switch field?.kind {
        case .text:
            SuggestField(text: $rule.value, values: field?.values, label: Filters.label(rule.field))
                .frame(maxWidth: .infinity)
        case .source:
            RuleSelectBox(
                options: sourceOptions,
                value: $rule.value,
                label: Filters.label(rule.field)
            )
            .frame(maxWidth: .infinity)
        case .playlist:
            RuleSelectBox(
                options: Store.shared.playlists.map { RuleSelectOption(value: String($0.id), label: $0.name) },
                value: $rule.value,
                label: Filters.label(rule.field)
            )
            .frame(maxWidth: .infinity)
        case .range:
            let low = rule.op != "until" && rule.op != "at_most"
            let high = rule.op != "from" && rule.op != "at_least"
            HStack(spacing: 6) {
                if low {
                    SuggestField(text: $rule.from, numeric: true, label: tr("range_from"))
                        .frame(maxWidth: 72)
                }
                if low && high {
                    Text(tr("range_to"))
                        .font(.system(size: 13))
                        .foregroundStyle(Theme.text3)
                }
                if high {
                    SuggestField(text: $rule.to, numeric: true, label: tr("range_until"))
                        .frame(maxWidth: 72)
                }
                Spacer(minLength: 0)
            }
            .frame(maxWidth: .infinity)
        case .period:
            HStack(spacing: 6) {
                SuggestField(text: $rule.amount, numeric: true, label: tr("period_amount"))
                    .frame(width: 56)
                RuleSelectBox(
                    options: Filters.units.map { RuleSelectOption(value: $0, label: tr("unit_" + $0)) },
                    value: $rule.unit,
                    muted: true,
                    label: tr("period_unit")
                )
                .frame(maxWidth: .infinity)
            }
            .frame(maxWidth: .infinity)
        case nil:
            Spacer(minLength: 0)
        }
    }

    /// The sources under their kinds.
    private var sourceOptions: [RuleSelectOption] {
        let sources = FilterCatalog.shared.sources
        return FilterCatalog.kinds.flatMap { kind -> [RuleSelectOption] in
            let ofKind = sources.filter { $0.kind == kind }
            guard !ofKind.isEmpty else { return [] }
            return [RuleSelectOption.heading(FilterCatalog.heading(kind))]
                + ofKind.map { RuleSelectOption(value: $0.value, label: $0.label) }
        }
    }
}

/// Rules that count as one (`qml/RuleGroup.qml`): a box with [All/Any/None] "of" and a "×",
/// its rules, and "+ Add rule to group".
struct RuleGroupView: View {
    @Binding var group: FilterGroup
    let remove: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 8) {
                RuleSelectBox(
                    options: Filters.groupValues.map { RuleSelectOption(value: $0, label: tr("group_" + $0)) },
                    value: $group.group,
                    height: 28,
                    radius: 6,
                    weight: .semibold,
                    fontSize: 13,
                    label: tr("group_combines")
                )
                .fixedSize()
                Text(tr("group_of"))
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.text2)
                Spacer(minLength: 0)
                RuleRemoveButton(label: tr("remove_group"), action: remove)
            }
            .frame(height: 32)
            ForEach($group.rules) { $rule in
                RuleRowView(rule: $rule) {
                    group.rules.removeAll { $0.id == rule.id }
                }
            }
            RuleFlatButton(title: tr("add_rule_to_group")) {
                group.rules.append(Filters.newRule())
            }
        }
        .padding(10)
        .background(RoundedRectangle(cornerRadius: 10, style: .continuous).fill(Theme.field))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(Theme.line2, lineWidth: 1))
    }
}
