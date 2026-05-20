#[derive(Debug, Clone, Copy, Default)]
pub struct SourceLocation {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<ASTNode>,
    pub lines: Vec<u32>,
    pub source_type: SourceType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceType {
    Script,
    Module,
}

#[derive(Debug, Clone)]
pub enum ASTNode {
    BlockStatement(BlockStatement),
    VariableDeclaration(VariableDeclaration),
    FunctionDeclaration(FunctionDeclaration),
    ClassDeclaration(ClassDeclaration),
    IfStatement(IfStatement),
    ForStatement(ForStatement),
    ForInStatement(ForInStatement),
    ForOfStatement(ForOfStatement),
    WhileStatement(WhileStatement),
    DoWhileStatement(DoWhileStatement),
    SwitchStatement(SwitchStatement),
    TryStatement(TryStatement),
    WithStatement(WithStatement),
    ReturnStatement(ReturnStatement),
    ThrowStatement(ThrowStatement),
    BreakStatement(BreakStatement),
    ContinueStatement(ContinueStatement),
    LabelledStatement(LabelledStatement),
    DebuggerStatement,
    EmptyStatement,

    ImportDeclaration(ImportDeclaration),
    ExportNamedDeclaration(ExportNamedDeclaration),
    ExportDefaultDeclaration(ExportDefaultDeclaration),
    ExportAllDeclaration(ExportAllDeclaration),

    ExpressionStatement(ExpressionStatement),
}

#[derive(Debug, Clone)]
pub enum Expression {
    Identifier(Identifier),
    PrivateIdentifier(String),
    Literal(Literal),
    This,
    Super,
    ArrayExpression(ArrayExpression),
    ObjectExpression(ObjectExpression),
    FunctionExpression(FunctionExpression),
    ClassExpression(ClassExpression),
    TemplateLiteral(TemplateLiteral),
    RegExpLiteral(RegExpLiteral),

    UnaryExpression(UnaryExpression),
    UpdateExpression(UpdateExpression),
    BinaryExpression(BinaryExpression),
    LogicalExpression(LogicalExpression),
    ConditionalExpression(ConditionalExpression),
    AssignmentExpression(AssignmentExpression),
    SequenceExpression(SequenceExpression),

    MemberExpression(MemberExpression),
    CallExpression(CallExpression),
    NewExpression(NewExpression),
    OptionalMemberExpression(OptionalMemberExpression),
    OptionalCallExpression(OptionalCallExpression),

    ArrowFunction(ArrowFunction),
    AwaitExpression(AwaitExpression),
    YieldExpression(YieldExpression),
    SpreadElement(SpreadElement),
    TaggedTemplateExpression(TaggedTemplateExpression),
    MetaProperty(MetaProperty),

    ArrayPattern(ArrayPattern),
    ObjectPattern(ObjectPattern),
    RestElement(RestElement),
    AssignmentPattern(AssignmentPattern),
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Number(f64),
    String(String, bool), 
    Boolean(bool),
    Null,
    Undefined,
    BigInt(String),
    LegacyOctal(i64),
}

#[derive(Debug, Clone)]
pub struct ArrayExpression {
    pub elements: Vec<Option<ArrayElement>>,
}

#[derive(Debug, Clone)]
pub enum ArrayElement {
    Expression(Expression),
    Spread(Expression),
}

#[derive(Debug, Clone)]
pub struct ObjectExpression {
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone)]
pub enum Property {
    Property {
        key: PropertyKey,
        value: Box<Expression>,
        computed: bool,
        shorthand: bool,
        method: bool,
        getter: bool,
        setter: bool,
    },

    SpreadElement(Expression),
}

#[derive(Debug, Clone)]
pub enum PropertyKey {
    Identifier(String),
    Literal(Literal),
    Computed(Box<Expression>),
    PrivateIdentifier(String),
}

#[derive(Debug, Clone)]
pub struct FunctionExpression {
    pub name: Option<String>,
    pub params: Vec<Parameter>,
    pub body: BlockStatement,
    pub generator: bool,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub struct ClassExpression {
    pub name: Option<String>,
    pub super_class: Option<Box<Expression>>,
    pub body: ClassBody,
}

#[derive(Debug, Clone)]
pub struct TemplateLiteral {
    pub quasis: Vec<TemplateElement>,
    pub expressions: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct TemplateElement {
    pub value: String,
    pub tail: bool,
}

#[derive(Debug, Clone)]
pub struct RegExpLiteral {
    pub pattern: String,
    pub flags: String,
}

#[derive(Debug, Clone)]
pub struct UnaryExpression {
    pub op: UnaryOp,
    pub argument: Box<Expression>,
    pub prefix: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Minus,
    Plus,
    Not,
    BitNot,
    TypeOf,
    Void,
    Delete,
}

#[derive(Debug, Clone)]
pub struct UpdateExpression {
    pub op: UpdateOp,
    pub argument: Box<Expression>,
    pub prefix: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateOp {
    Increment,
    Decrement,
}

#[derive(Debug, Clone)]
pub struct BinaryExpression {
    pub op: BinaryOp,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Neq,
    StrictEq,
    StrictNeq,
    Lt,
    Lte,
    Gt,
    Gte,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,
    In,
    InstanceOf,
}

#[derive(Debug, Clone)]
pub struct LogicalExpression {
    pub op: LogicalOp,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone)]
pub enum LogicalOp {
    And,
    Or,
    NullishCoalescing,
}

#[derive(Debug, Clone)]
pub struct ConditionalExpression {
    pub test: Box<Expression>,
    pub consequent: Box<Expression>,
    pub alternate: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct AssignmentExpression {
    pub op: AssignOp,
    pub left: AssignmentTarget,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    PowAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
    UShrAssign,
    LogicalAndAssign,
    LogicalOrAssign,
    NullishAssign,
}

#[derive(Debug, Clone)]
pub enum AssignmentTarget {
    Identifier(String),
    MemberExpression(MemberExpression),
    OptionalMemberExpression(OptionalMemberExpression),
    ComputedMember(Box<Expression>, Box<Expression>),
    ObjectPattern(ObjectPattern),
    ArrayPattern(ArrayPattern),
    RestElement(RestElement),
    AssignmentPattern(AssignmentPattern),
}

#[derive(Debug, Clone)]
pub struct SequenceExpression {
    pub expressions: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct MemberExpression {
    pub object: Box<Expression>,
    pub property: MemberProperty,
    pub computed: bool,
}

#[derive(Debug, Clone)]
pub enum MemberProperty {
    Identifier(String),
    Computed(Box<Expression>),
    PrivateIdentifier(String),
}

#[derive(Debug, Clone)]
pub struct OptionalMemberExpression {
    pub object: Box<Expression>,
    pub property: MemberProperty,
    pub computed: bool,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct CallExpression {
    pub callee: Box<Expression>,
    pub arguments: Vec<Argument>,
}

#[derive(Debug, Clone)]
pub enum Argument {
    Expression(Expression),
    Spread(Expression),
}

#[derive(Debug, Clone)]
pub struct OptionalCallExpression {
    pub callee: Box<Expression>,
    pub arguments: Vec<Argument>,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct NewExpression {
    pub callee: Box<Expression>,
    pub arguments: Vec<Argument>,
}

#[derive(Debug, Clone)]
pub struct ArrowFunction {
    pub params: Vec<Parameter>,
    pub body: ArrowBody,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub enum ArrowBody {
    Expression(Box<Expression>),
    Block(BlockStatement),
}

#[derive(Debug, Clone)]
pub struct AwaitExpression {
    pub argument: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct YieldExpression {
    pub argument: Option<Box<Expression>>,
    pub delegate: bool,
}

#[derive(Debug, Clone)]
pub struct SpreadElement {
    pub argument: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct TaggedTemplateExpression {
    pub tag: Box<Expression>,
    pub quasi: TemplateLiteral,
}

#[derive(Debug, Clone)]
pub struct MetaProperty {
    pub meta: String,
    pub property: String,
}

#[derive(Debug, Clone)]
pub struct ArrayPattern {
    pub elements: Vec<Option<PatternElement>>,
}

#[derive(Debug, Clone)]
pub enum PatternElement {
    Pattern(AssignmentTarget),
    RestElement(RestElement),
    AssignmentPattern(AssignmentPattern),
}

#[derive(Debug, Clone)]
pub struct ObjectPattern {
    pub properties: Vec<ObjectPatternProperty>,
}

#[derive(Debug, Clone)]
pub enum ObjectPatternProperty {
    Property {
        key: PropertyKey,
        value: AssignmentTarget,
        computed: bool,
        shorthand: bool,
    },
    RestElement(RestElement),
}

#[derive(Debug, Clone)]
pub struct RestElement {
    pub argument: Box<AssignmentTarget>,
}

#[derive(Debug, Clone)]
pub struct AssignmentPattern {
    pub left: Box<AssignmentTarget>,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone)]
pub enum Parameter {
    Identifier(String),
    Pattern(BindingPattern),
    AssignmentPattern(AssignmentPattern),
    RestElement(RestElement),
}

#[derive(Debug, Clone)]
pub struct ExpressionStatement {
    pub expression: Expression,
}

#[derive(Debug, Clone)]
pub struct BlockStatement {
    pub body: Vec<ASTNode>,
    pub lines: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct VariableDeclaration {
    pub kind: VariableKind,
    pub declarations: Vec<VariableDeclarator>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VariableKind {
    Var,
    Let,
    Const,
}

#[derive(Debug, Clone)]
pub struct VariableDeclarator {
    pub id: BindingPattern,
    pub init: Option<Expression>,
}

#[derive(Debug, Clone)]
pub enum BindingPattern {
    Identifier(String),
    ArrayPattern(ArrayPattern),
    ObjectPattern(ObjectPattern),
    AssignmentPattern(AssignmentPattern),
}

#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: BlockStatement,
    pub generator: bool,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub struct ClassDeclaration {
    pub name: String,
    pub super_class: Option<Box<Expression>>,
    pub body: ClassBody,
}

#[derive(Debug, Clone)]
pub struct ClassBody {
    pub elements: Vec<ClassElement>,
}

#[derive(Debug, Clone)]
pub enum ClassElement {
    Method(ClassMethod),
    StaticBlock(StaticBlock),
    Property(ClassProperty),
}

#[derive(Debug, Clone)]
pub struct ClassMethod {
    pub name: PropertyKey,
    pub params: Vec<Parameter>,
    pub body: BlockStatement,
    pub kind: MethodKind,
    pub is_async: bool,
    pub generator: bool,
    pub is_static: bool,
    pub is_private: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MethodKind {
    Constructor,
    Method,
    Get,
    Set,
}

#[derive(Debug, Clone)]
pub struct ClassProperty {
    pub name: PropertyKey,
    pub value: Option<Expression>,
    pub is_static: bool,
    pub is_private: bool,
}

#[derive(Debug, Clone)]
pub struct StaticBlock {
    pub body: Vec<ASTNode>,
    pub lines: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct IfStatement {
    pub test: Expression,
    pub consequent: Box<ASTNode>,
    pub alternate: Option<Box<ASTNode>>,
}

#[derive(Debug, Clone)]
pub struct ForStatement {
    pub init: Option<ForInit>,
    pub test: Option<Expression>,
    pub update: Option<Expression>,
    pub body: Box<ASTNode>,
}

#[derive(Debug, Clone)]
pub enum ForInit {
    VariableDeclaration(VariableDeclaration),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct ForInStatement {
    pub left: ForInOfLeft,
    pub right: Expression,
    pub body: Box<ASTNode>,
}

#[derive(Debug, Clone)]
pub struct ForOfStatement {
    pub left: ForInOfLeft,
    pub right: Expression,
    pub body: Box<ASTNode>,
    pub is_await: bool,
}

#[derive(Debug, Clone)]
pub enum ForInOfLeft {
    VariableDeclaration(VariableDeclaration),
    AssignmentTarget(AssignmentTarget),
}

#[derive(Debug, Clone)]
pub struct WhileStatement {
    pub test: Expression,
    pub body: Box<ASTNode>,
}

#[derive(Debug, Clone)]
pub struct DoWhileStatement {
    pub body: Box<ASTNode>,
    pub test: Expression,
}

#[derive(Debug, Clone)]
pub struct SwitchStatement {
    pub discriminant: Expression,
    pub cases: Vec<SwitchCase>,
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub test: Option<Expression>,
    pub consequent: Vec<ASTNode>,
}

#[derive(Debug, Clone)]
pub struct TryStatement {
    pub block: BlockStatement,
    pub handler: Option<CatchClause>,
    pub finalizer: Option<BlockStatement>,
}

#[derive(Debug, Clone)]
pub struct CatchClause {
    pub param: Option<BindingPattern>,
    pub body: BlockStatement,
}

#[derive(Debug, Clone)]
pub struct WithStatement {
    pub object: Expression,
    pub body: Box<ASTNode>,
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub argument: Option<Expression>,
}

#[derive(Debug, Clone)]
pub struct ThrowStatement {
    pub argument: Expression,
}

#[derive(Debug, Clone)]
pub struct BreakStatement {
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContinueStatement {
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LabelledStatement {
    pub label: String,
    pub body: Box<ASTNode>,
}

#[derive(Debug, Clone)]
pub struct ImportDeclaration {
    pub specifiers: Vec<ImportSpecifier>,
    pub source: String,
}

#[derive(Debug, Clone)]
pub enum ImportSpecifier {
    Named { imported: String, local: String },

    Default { local: String },

    Namespace { local: String },
}

#[derive(Debug, Clone)]
pub struct ExportNamedDeclaration {
    pub declaration: Option<ExportDeclaration>,
    pub specifiers: Vec<ExportSpecifier>,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ExportDeclaration {
    Variable(VariableDeclaration),
    Function(FunctionDeclaration),
    Class(ClassDeclaration),
}

#[derive(Debug, Clone)]
pub struct ExportSpecifier {
    pub exported: String,
    pub local: String,
}

#[derive(Debug, Clone)]
pub struct ExportDefaultDeclaration {
    pub declaration: ExportDefaultDeclarationKind,
}

#[derive(Debug, Clone)]
pub enum ExportDefaultDeclarationKind {
    Function(FunctionDeclaration),
    Class(ClassDeclaration),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct ExportAllDeclaration {
    pub source: String,
    pub exported: Option<String>,
}
