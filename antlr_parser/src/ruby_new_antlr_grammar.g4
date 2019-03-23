grammar ruby;


// begin Ruby Program
ruby: program;

program 
    :   ( class_def (NEWLINE)+  | statement (NEWLINE)+)* 
    ;

class_name
    :   IDENTIFIER 
    ;

class_def
    :   ' class ' 
        class_name
        (NEWLINE  | '<' class_parent ('<' class_parent)* NEWLINE )
        class_body
        ' end'
    ;

class_parent
    :   IDENTIFIER
    ;

class_body
    : ( (statement NEWLINE  ) | ( method_def NEWLINE) )+
    ;

method_def
    :   ' def '
        (class_name '.')? method_name
        ((method_par)*
        |( '(' method_par (',' method_par )* ')' )* )
        ( NEWLINE )+
        method_body
        ' end'
    ;

method_name
    :   IDENTIFIER
    ;

method_par
    :   IDENTIFIER
    ;

method_body
    : (statement NEWLINE  )*
    ;

// Instruction in the program body
statement
    :   ' if ' 
          '(' c=condition ')' NEWLINE b=block ' end' 
	| ' break if '
          '(' c=condition ')' 
	| ' loop ' 'do' NEWLINE loopBody=block ' end'
	| // yield
          ' yield '
          '(' operando ')' 
        | // return
          ' return ' (operando  | '(' operando ')' )
        | // return
          ' return ' (booleani | '(' booleani ')' )
        | // puts
          ' puts ' statement 
	| // id = (number | id | string)
          (DAT IDENTIFIER) 
          '=' (operando | '(' operando_par ')' )
        | // id = id (id, id, ...)
          (DAT IDENTIFIER)
          '=' 
          (class_name '.' method_name)
          ( ('(' ')') | ( '(' 
          operando 
          (',' 
          operando )* 
          ')' ) )?
        | // id = id (id, id, ...)
          (DAT IDENTIFIER)
          '=' 
          method_name
          ( ('(' ')') | ( '(' 
          operando
          (',' 
          operando )* 
          ')' ) )?
        | // id = (number | id | string)
	  (AT IDENTIFIER) 
          '=' (operando | '(' operando ')' )
        | // id = id (id, id, ...)
          (AT IDENTIFIER)
          '=' 
          (class_name '.' method_name)
          ( ('(' ')') | ( '(' 
          operando 
          (',' 
          operando )* 
          ')' ) )?
        | // id = id (id, id, ...) 
          (AT IDENTIFIER)
          '=' 
          method_name
          ( ('(' ')') | ( '(' 
          operando 
          (',' 
          operando )* 
          ')' ) )?
        | // id = (number | id | string)
          IDENTIFIER  
          '=' (operando  | '(' operando ')' )
        | // id = id (id, id, ...)
          IDENTIFIER 
          '='
          (class_name '.' method_name)
          ( ('(' ')')| ( '(' 
          operando 
          (',' 
          operando )* 
          ')' ) )?
        | // id = id (id, id, ...)
          IDENTIFIER 
          '=' 
          method_name
          ( ('(' ')')  | ( '(' 
          operando 
          (',' 
          operando )* 
          ')' ) )?
        | operando 
    ;

block
    :   ( s=statement NEWLINE  )+ 
    ;
	   
// condition or operation 
condition
    :   // condition operator (IDENTIFIER | NUMBER)
          booleani 
        | booleani (' & ' 
                    | ' | ') 
          booleani  
        | '(' condition ')' 
        | booleani ' | '
          condition 
        | booleani ' & ' booleani (' & '
                               | ' | ') 
        condition
    ;

// math instruction
instruction 
    :     '(' operando_par ')'  ( '/' operando
                                | ' * '  operando  
                                | PLUS  operando 
                                | '-' operando  )?
        | '(' instruction_par ')' ( '/'  operando
                                  | ' * ' operando
                                  | PLUS operando
                                  | '-' operando )?
        | (DAT IDENTIFIER) ( '/' operando 
                           | ' * ' operando 
                           | PLUS operando 
                           | '-' operando  )
        | (AT IDENTIFIER) ( '/' operando 
                          | ' * ' operando 
                          | PLUS operando 
                          | '-' operando )
        | IDENTIFIER ( '/' operando 
                     | ' * ' operando
                     | PLUS operando 
                     | '-' operando  )
        | //NUMBER operator NUMBER
          NUMBER ( '/' operando
                   | ' * ' operando 
                   | PLUS operando
                   | '-' operando)
    ;

instruction_par
    :   instruction  
    ;

operando_par
    :   operando  
    ;

operando
    :     NUMBER 
        | (DAT IDENTIFIER) 
        | (AT IDENTIFIER) 
        | IDENTIFIER 
        | class_name '.' 'new' ( '.' method_name  ('.' method_name)* )?
          ( '(' 
          operando 
          (',' 
          operando 
          )* ')' )?
        | // id((id | number),(id|number)*) do |id|
          (class_name '.' method_name)
          ( ('(' ')')  | ( '(' 
          operando 
          (',' 
          operando  )* 
          ')' ) )?
          ( 'do ' '|' IDENTIFIER 
                 '|' NEWLINE 
                  block
                  ' end')?
        | // id((id | number),(id|number)*) do |id|
          method_name
          ( ('(' ')')  | ( '(' 
          operando
          (',' 
          operando )* 
          ')' ) )?
	  ( ' do ' '|' IDENTIFIER 
                 '|' NEWLINE 
                 block
                 ' end')?
        | instruction 
        | '(' instruction_par ')' 
    ;

booleani
    :     ' true ' 
        | ' false '
        | operando ( '<' operando
                   | '<=' operando
                   | '>=' operando 
                   | '>'  operando
                   | '==' operando  )
      
    ;




WS	: ' '
	;

LPAREN  : '('
	;

RPAREN  : ')'
	;

LT      : '<'
        ;
LE      : '<='
        ;
GE      : '>='
        ;
GT      : '>'
        ;
EGUAL   : '=='
        ;
DIV     : '/'
        ;
MUL     : '*'
        ;
ASSIGN  : '='
        ;
PLUS    : '+'
        ;
OR      : '|' 
        ;  
AND     : '&'
        ;
SUB     : '-'
        ;
MOD     : '%'
        ;
NUMBER  : ('0'..'9')+ 
        ;
POINT   : ('.')+
        ;
AT      : '@'
        ;
DAT     : '@@'
        ;
	 
// Id
IDENTIFIER : ('a'..'z'|'A'..'Z')+ (NUMBER)*
           ;

// new line
NEWLINE : '\r\n'  | '\r'  | '\n';
