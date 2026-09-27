use topcoat::{
    Result,
    view::{Attributes, Child, View, class, component, view},
};

/// A table inside a horizontally scrollable container.
///
/// Pass table sections as children. `attrs` are forwarded to the `<table>`, with extra
/// classes added to its classes.
///
/// ```ignore
/// view! {
///     table(
///         table_header(
///             table_row(
///                 table_head("Environment")
///                 table_head("Status")
///             )
///         )
///         table_body(
///             table_row(
///                 table_cell("production")
///                 table_cell("Live")
///             )
///         )
///     )
/// }
/// ```
#[component]
pub async fn table(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <div class="w-full overflow-x-auto">
            <table
                class=(class!(
                    "w-full caption-bottom border-collapse text-body",
                    attrs.remove("class"),
                ))
                (attrs)
            >
                (child)
            </table>
        </div>
    })
}

/// The heading section of a [`table`], holding the row of column headers.
#[component]
pub async fn table_header(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <thead class=(class!("[&_tr]:border-b", attrs.remove("class"))) (attrs)>
            (child)
        </thead>
    })
}

/// The body of a table, containing data rows.
#[component]
pub async fn table_body(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <tbody
            class=(class!("[&_tr:last-child]:border-0", attrs.remove("class")))
            (attrs)
        >
            (child)
        </tbody>
    })
}

/// The closing section of a [`table`], for totals and other summaries.
#[component]
pub async fn table_footer(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <tfoot
            class=(class!(
                "border-t border-line bg-subtle font-medium [&>tr]:last:border-b-0",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </tfoot>
    })
}

/// A table row with a separator and hover styling.
#[component]
pub async fn table_row(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <tr
            class=(class!(
                "border-b border-line aria-selected:bg-subtle data-[selected=true]:bg-subtle",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </tr>
    })
}

/// A column header in a [`table_header`]'s row.
#[component]
pub async fn table_head(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <th
            class=(class!(
                "px-3 py-2 text-left align-middle text-caption font-medium whitespace-nowrap \
                 text-muted",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </th>
    })
}

/// One cell of a [`table_row`].
#[component]
pub async fn table_cell(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <td
            class=(class!("px-3 py-3 align-middle", attrs.remove("class")))
            (attrs)
        >
            (child)
        </td>
    })
}

/// A caption describing the table.
#[component]
pub async fn table_caption(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <caption
            class=(class!("mt-4 text-body text-muted", attrs.remove("class")))
            (attrs)
        >
            (child)
        </caption>
    })
}
